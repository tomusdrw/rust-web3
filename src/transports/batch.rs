//! Batching Transport

use std::mem;
use std::collections::BTreeMap;
use std::sync::Arc;
use futures::{self, future, Future};
use futures::sync::oneshot;
use parking_lot::Mutex;
use rpc;
use transports::Result;
use {BatchTransport, Transport, Error as RpcError, ErrorKind, RequestId};

type Pending = oneshot::Sender<Result<rpc::Value>>;
type PendingRequests = Arc<Mutex<BTreeMap<RequestId, Pending>>>;

/// Transport allowing to batch queries together.
#[derive(Debug)]
pub struct Batch<T> {
  transport: T,
  pending: PendingRequests,
  batch: Mutex<Vec<(RequestId, rpc::Call)>>,
}

impl<T> Batch<T> where
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

impl<T> Transport for Batch<T> where
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
    future::JoinAll<Vec<::std::result::Result<(), Result<rpc::Value>>>>,
    Result<Vec<Result<rpc::Value>>>
  ),
  Done,
}

/// A result of submitting a batch request.
/// Returns the results of all requests within the batch.
pub struct BatchFuture<T> {
  state: BatchState<T>,
  pending: PendingRequests,
}

impl<T: Future<Item=Vec<Result<rpc::Value>>, Error=RpcError>> Future for BatchFuture<T> {
  type Item = Vec<Result<rpc::Value>>;
  type Error = RpcError;

  fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
    loop {
      match mem::replace(&mut self.state, BatchState::Done) {
        BatchState::SendingBatch(mut batch, ids) => {
          let res = match batch.poll() {
            Ok(futures::Async::NotReady) => {
              self.state = BatchState::SendingBatch(batch, ids);
              return Ok(futures::Async::NotReady);
            },
            Ok(futures::Async::Ready(v)) => Ok(v),
            Err(err) => Err(err),
          };

          let mut pending = self.pending.lock();
          let sending = ids.into_iter()
              .enumerate()
              .filter_map(|(idx, request_id)| {
                pending.remove(&request_id).map(|rx| {
                  match res {
                    Ok(ref results) if results.len() > idx => {
                      rx.send(results[idx].clone())
                    },
                    Err(ref err) => rx.send(Err(err.clone())),
                    _ => rx.send(Err(ErrorKind::Internal.into())),
                  }
                })
              })
              .collect::<Vec<_>>();

          self.state = BatchState::Resolving(
            future::join_all(sending),
            res,
          );
        },
        BatchState::Resolving(mut all, res) => {
          if let Ok(futures::Async::NotReady) = all.poll() {
            self.state = BatchState::Resolving(all, res);
            return Ok(futures::Async::NotReady);
          }

          return Ok(futures::Async::Ready(res?));
        },
        BatchState::Done => {
          panic!("Poll after Ready.");
        }
      };
    }
  }
}

/// Result of calling a single method that will be part of the batch.
/// Converts `oneshot::Receiver` error into `RpcError::Internal`
pub struct SingleResult(oneshot::Receiver<Result<rpc::Value>>);

impl Future for SingleResult {
  type Item = rpc::Value;
  type Error = RpcError;

  fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
    let res = try_ready!(self.0.poll().map_err(|_| RpcError::from(ErrorKind::Internal)));
    res.map(futures::Async::Ready)
  }

}
