//! Batching Transport

use std::mem;
use std::collections::BTreeMap;
use std::sync::Arc;
use futures::{self, future, Future, BoxFuture};
use futures::sync::oneshot;
use parking_lot::Mutex;
use rpc;
use {BatchTransport, Transport, Error as RpcError, RequestId};

type Pending = oneshot::Sender<Result<rpc::Value, RpcError>>;

pub struct Batch<T> {
  transport: T,
  pending: Arc<Mutex<BTreeMap<RequestId, Pending>>>,
  batch: Mutex<Vec<(RequestId, rpc::Call)>>,
}

impl<T> Batch<T> where
  T: BatchTransport,
{
  /// Sends all requests as a batch.
  pub fn submit_batch(&self) -> BoxFuture<Vec<rpc::Value>, RpcError> {
    let batch = mem::replace(&mut *self.batch.lock(), vec![]);
    let ids = batch.iter().map(|&(id, _)| id).collect::<Vec<_>>();
    let pending = self.pending.clone();
    self.transport.send_batch(batch)
      .then(move |res| {
        let mut pending = pending.lock();
        let sending = ids.into_iter()
            .enumerate()
            .filter_map(|(idx, request_id)| {
              pending.remove(&request_id).map(|rx| {
                match res {
                  Ok(ref results) if results.len() > idx => {
                    rx.send(Ok(results[idx].clone()))
                  },
                  Err(ref err) => rx.send(Err(err.clone())),
                  _ => rx.send(Err(RpcError::Internal)),
                }
              })
            })
            .collect::<Vec<_>>();

        future::join_all(sending)
          .then(|_| futures::done(res))
      })
      //TODO [ToDr] Don't box here!
      .boxed()
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


pub struct SingleResult(oneshot::Receiver<Result<rpc::Value, RpcError>>);

impl Future for SingleResult {
  type Item = rpc::Value;
  type Error = RpcError;

  fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
    let res = try_ready!(self.0.poll().map_err(|_| RpcError::Internal));
    res.map(futures::Async::Ready)
  }

}
