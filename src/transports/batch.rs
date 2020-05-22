//! Batching Transport

use crate::rpc;
use crate::error::{self, Error};
use crate::{BatchTransport, RequestId, Transport};
use futures::sync::oneshot;
use futures::{self, future, Future};
use parking_lot::Mutex;
use std::collections::BTreeMap;
use std::mem;
use std::sync::Arc;

type Pending = oneshot::Sender<error::Result<rpc::Value>>;
type PendingRequests = Arc<Mutex<BTreeMap<RequestId, Pending>>>;

/// Transport allowing to batch queries together.
#[derive(Debug, Clone)]
pub struct Batch<T> {
    transport: T,
    pending: PendingRequests,
    batch: Arc<Mutex<Vec<(RequestId, rpc::Call)>>>,
}

impl<T> Batch<T>
where
    T: BatchTransport,
{
    /// Creates new Batch transport given existing transport supporing batch requests.
    pub fn new(transport: T) -> Self {
        Batch {
            transport,
            pending: Default::default(),
            batch: Default::default(),
        }
    }

    /// Sends all requests as a batch.
    pub fn submit_batch(&self) -> BatchFuture<T::Batch> {
        let batch = mem::replace(&mut *self.batch.lock(), vec![]);
        let ids = batch.iter().map(|&(id, _)| id).collect::<Vec<_>>();

        let batch = self.transport.send_batch(batch);
        let pending = self.pending.clone();

        BatchFuture {
            state: BatchState::SendingBatch(batch, ids),
            pending,
        }
    }
}

impl<T> Transport for Batch<T>
where
    T: BatchTransport,
{
    type Out = SingleResult;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        self.transport.prepare(method, params)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        let (tx, rx) = futures::oneshot();
        self.pending.lock().insert(id, tx);
        self.batch.lock().push((id, request));

        SingleResult(rx)
    }
}

enum BatchState<T> {
    SendingBatch(T, Vec<RequestId>),
    Resolving(
        future::JoinAll<Vec<Result<(), error::Result<rpc::Value>>>>,
        error::Result<Vec<error::Result<rpc::Value>>>,
    ),
    Done,
}

/// A result of submitting a batch request.
/// Returns the results of all requests within the batch.
pub struct BatchFuture<T> {
    state: BatchState<T>,
    pending: PendingRequests,
}

impl<T: Future<Output = error::Result<Vec<error::Result<rpc::Value>>>>> Future for BatchFuture<T> {
    type Output = error::Result<Vec<error::Result<rpc::Value>>>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> futures::Poll<Self::Output> {
        loop {
            match mem::replace(&mut self.state, BatchState::Done) {
                BatchState::SendingBatch(mut batch, ids) => {
                    let res = match batch.poll(ctx) {
                        Ok(futures::Poll::Pending) => {
                            self.state = BatchState::SendingBatch(batch, ids);
                            return Ok(futures::Poll::Pending);
                        }
                        Ok(futures::Poll::Ready(v)) => Ok(v),
                        Err(err) => Err(err),
                    };

                    let mut pending = self.pending.lock();
                    let sending = ids
                        .into_iter()
                        .enumerate()
                        .filter_map(|(idx, request_id)| {
                            pending.remove(&request_id).map(|rx| match res {
                                Ok(ref results) if results.len() > idx => rx.send(results[idx].clone()),
                                Err(ref err) => rx.send(Err(err.clone())),
                                _ => rx.send(Err(Error::Internal)),
                            })
                        })
                        .collect::<Vec<_>>();

                    self.state = BatchState::Resolving(future::join_all(sending), res);
                }
                BatchState::Resolving(mut all, res) => {
                    if let Ok(futures::Poll::Pending) = all.poll(ctx) {
                        self.state = BatchState::Resolving(all, res);
                        return Ok(futures::Poll::Pending);
                    }

                    return Ok(futures::Poll::Ready(res?));
                }
                BatchState::Done => {
                    panic!("Poll after Ready.");
                }
            };
        }
    }
}

/// Result of calling a single method that will be part of the batch.
/// Converts `oneshot::Receiver` error into `Error::Internal`
pub struct SingleResult(oneshot::Receiver<error::Result<rpc::Value>>);

impl Future for SingleResult {
    type Output = error::Result<rpc::Value>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> futures::Poll<Self::Output> {
        let res = ready!(self.0.poll(ctx).map_err(|_| Error::Internal));
        res.map(futures::Poll::Ready)
    }
}
