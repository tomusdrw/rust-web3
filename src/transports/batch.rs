//! Batching Transport

use crate::{
    error::{self, Error},
    rpc, BatchTransport, RequestId, Transport,
};
use futures::{
    channel::oneshot,
    task::{Context, Poll},
    Future, FutureExt,
};
use parking_lot::Mutex;
use std::{collections::BTreeMap, mem, pin::Pin, sync::Arc};

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
    pub fn submit_batch(&self) -> impl Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> {
        let batch = mem::replace(&mut *self.batch.lock(), vec![]);
        let ids = batch.iter().map(|&(id, _)| id).collect::<Vec<_>>();

        let batch = self.transport.send_batch(batch);
        let pending = self.pending.clone();

        async move {
            let res = batch.await;
            let mut pending = pending.lock();
            for (idx, request_id) in ids.into_iter().enumerate() {
                if let Some(rx) = pending.remove(&request_id) {
                    // Ignore sending error
                    let _ = match res {
                        Ok(ref results) if results.len() > idx => rx.send(results[idx].clone()),
                        Err(ref err) => rx.send(Err(err.clone())),
                        _ => rx.send(Err(Error::Internal)),
                    };
                }
            }
            res
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
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);
        self.batch.lock().push((id, request));

        SingleResult(rx)
    }
}

/// Result of calling a single method that will be part of the batch.
/// Converts `oneshot::Receiver` error into `Error::Internal`
pub struct SingleResult(oneshot::Receiver<error::Result<rpc::Value>>);

impl Future for SingleResult {
    type Output = error::Result<rpc::Value>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        Poll::Ready(ready!(self.0.poll_unpin(ctx)).map_err(|_| Error::Internal)?)
    }
}
