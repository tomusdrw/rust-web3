//! `Eth` namespace, subscriptions

use std::fmt;
use std::marker::PhantomData;

use api::Namespace;
use futures::{Async, Future, Poll, Stream};
use helpers::{self, CallResult};
use rpc;
use serde;
use serde_json;
use types::{Address, BlockHeader, Filter, H256, Log};
use {DuplexTransport, Error};

/// `Eth` namespace, subscriptions
#[derive(Debug, Clone)]
pub struct EthSubscribe<T> {
  transport: T,
}

impl<T: DuplexTransport> Namespace<T> for EthSubscribe<T> {
  fn new(transport: T) -> Self
  where
    Self: Sized,
  {
    EthSubscribe { transport }
  }

  fn transport(&self) -> &T {
    &self.transport
  }
}

pub struct SubscriptionStream<T: DuplexTransport, I> {
  transport: T,
  id: String,
  rx: T::NotificationStream,
  _marker: PhantomData<I>,
}

impl<T: DuplexTransport, I> SubscriptionStream<T, I> {
  pub fn new(transport: T, id: String) -> Self {
    let rx = transport.subscribe(&id);
    SubscriptionStream {
      transport,
      id,
      rx,
      _marker: PhantomData,
    }
  }

  pub fn id(&self) -> &str {
    &self.id
  }
}

impl<T, I> Stream for SubscriptionStream<T, I>
where
  T: DuplexTransport,
  I: serde::de::DeserializeOwned,
{
  type Item = I;
  type Error = Error;

  fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
    match self.rx.poll() {
      Ok(Async::Ready(Some(x))) => serde_json::from_value(x)
        .map(Async::Ready)
        .map_err(Into::into),
      Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
      Ok(Async::NotReady) => Ok(Async::NotReady),
      Err(e) => Err(e),
    }
  }
}

impl<T: DuplexTransport, I> fmt::Debug for SubscriptionStream<T, I> {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{:?}", self.id())
  }
}

impl<T: DuplexTransport, I> Drop for SubscriptionStream<T, I> {
  fn drop(&mut self) {
    self.transport.unsubscribe(self.id());
  }
}

#[derive(Debug)]
pub struct SubscriptionResult<T: DuplexTransport, I> {
  transport: T,
  inner: CallResult<String, T::Out>,
  _marker: PhantomData<I>,
}

impl<T: DuplexTransport, I> SubscriptionResult<T, I> {
  pub fn new(transport: T, id_future: CallResult<String, T::Out>) -> Self {
    SubscriptionResult {
      transport,
      inner: id_future,
      _marker: PhantomData,
    }
  }
}

impl<T, I> Future for SubscriptionResult<T, I>
where
  T: DuplexTransport,
  I: serde::de::DeserializeOwned,
{
  type Item = (String, SubscriptionStream<T, I>);
  type Error = Error;

  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    match self.inner.poll() {
      Ok(Async::Ready(id)) => Ok(Async::Ready((
        id.clone(),
        SubscriptionStream::new(self.transport.clone(), id),
      ))),
      Ok(Async::NotReady) => Ok(Async::NotReady),
      Err(e) => Err(e),
    }
  }
}

impl<T: DuplexTransport> EthSubscribe<T> {
  /// Create a new heads subscription
  pub fn subscribe_new_heads(&self) -> SubscriptionResult<T, BlockHeader> {
    let subscription = helpers::serialize(&&"newHeads");
    let id_future = CallResult::new(self.transport.execute("eth_subscribe", vec![subscription]));
    SubscriptionResult::new(self.transport().clone(), id_future)
  }

  /// Create a logs subscription
  pub fn subscribe_logs(
    &self,
    filter: Filter
  ) -> SubscriptionResult<T, Log> {
    let subscription = helpers::serialize(&&"logs");
    let filter = helpers::serialize(&filter);
    let id_future = CallResult::new(self.transport.execute("eth_subscribe", vec![subscription, filter]));
    SubscriptionResult::new(self.transport().clone(), id_future)
  }

  pub fn subscribe_new_pending_transactions(&self) -> SubscriptionResult<T, H256> {
    let subscription = helpers::serialize(&&"newPendingTransactions");
    let id_future = CallResult::new(self.transport.execute("eth_subscribe", vec![subscription]));
    SubscriptionResult::new(self.transport().clone(), id_future)
  }

  pub fn subscribe_syncing(&self) -> SubscriptionResult<T, rpc::Value> {
    let subscription = helpers::serialize(&&"syncing");
    let id_future = CallResult::new(self.transport.execute("eth_subscribe", vec![subscription]));
    SubscriptionResult::new(self.transport().clone(), id_future)
  }

  pub fn unsubscribe(&self, id: &str) -> CallResult<bool, T::Out> {
    let id = helpers::serialize(&id);
    CallResult::new(self.transport.execute("eth_unsubscribe", vec![id]))
  }
}
