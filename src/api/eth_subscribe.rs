//! `Eth` namespace, subscriptions

use std::marker::PhantomData;

use api::Namespace;
use futures::{Async, Future, Poll, Stream};
use helpers::{self, CallFuture};
use serde;
use serde_json;
use types::{BlockHeader, Filter, H256, Log, SyncState};
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

/// ID of subscription returned from `eth_subscribe`
#[derive(Debug, Clone, Eq, Ord, PartialEq, PartialOrd)]
pub struct SubscriptionId(String);

impl From<String> for SubscriptionId {
    fn from(s: String) -> Self {
        SubscriptionId(s)
    }
}

/// Stream of notifications from a subscription
/// Given a type deserializable from rpc::Value and a subscription id, yields items of that type as
/// notifications are delivered.
#[derive(Debug)]
pub struct SubscriptionStream<T: DuplexTransport, I> {
    transport: T,
    id: SubscriptionId,
    rx: T::NotificationStream,
    _marker: PhantomData<I>,
}

impl<T: DuplexTransport, I> SubscriptionStream<T, I> {
    fn new(transport: T, id: SubscriptionId) -> Self {
        let rx = transport.subscribe(&id);
        SubscriptionStream {
            transport,
            id,
            rx,
            _marker: PhantomData,
        }
    }

    /// Return the ID of this subscription
    pub fn id(&self) -> &SubscriptionId {
        &self.id
    }

    /// Unsubscribe from the event represented by this stream
    pub fn unsubscribe(self) -> CallFuture<bool, T::Out> {
        let &SubscriptionId(ref id) = &self.id;
        let id = helpers::serialize(&id);
        CallFuture::new(self.transport.execute("eth_unsubscribe", vec![id]))
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

impl<T: DuplexTransport, I> Drop for SubscriptionStream<T, I> {
    fn drop(&mut self) {
        self.transport.unsubscribe(self.id());
    }
}

#[derive(Debug)]
pub struct SubscriptionResult<T: DuplexTransport, I> {
    transport: T,
    inner: CallFuture<String, T::Out>,
    _marker: PhantomData<I>,
}

impl<T: DuplexTransport, I> SubscriptionResult<T, I> {
    pub fn new(transport: T, id_future: CallFuture<String, T::Out>) -> Self {
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
    type Item = SubscriptionStream<T, I>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Ok(Async::Ready(id)) => Ok(Async::Ready(SubscriptionStream::new(
                self.transport.clone(),
                SubscriptionId(id),
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
        let id_future = CallFuture::new(self.transport.execute("eth_subscribe", vec![subscription]));
        SubscriptionResult::new(self.transport().clone(), id_future)
    }

    /// Create a logs subscription
    pub fn subscribe_logs(&self, filter: Filter) -> SubscriptionResult<T, Log> {
        let subscription = helpers::serialize(&&"logs");
        let filter = helpers::serialize(&filter);
        let id_future = CallFuture::new(
            self.transport
                .execute("eth_subscribe", vec![subscription, filter]),
        );
        SubscriptionResult::new(self.transport().clone(), id_future)
    }

    /// Create a pending transactions subscription
    pub fn subscribe_new_pending_transactions(&self) -> SubscriptionResult<T, H256> {
        let subscription = helpers::serialize(&&"newPendingTransactions");
        let id_future = CallFuture::new(self.transport.execute("eth_subscribe", vec![subscription]));
        SubscriptionResult::new(self.transport().clone(), id_future)
    }

    /// Create a sync status subscription
    pub fn subscribe_syncing(&self) -> SubscriptionResult<T, SyncState> {
        let subscription = helpers::serialize(&&"syncing");
        let id_future = CallFuture::new(self.transport.execute("eth_subscribe", vec![subscription]));
        SubscriptionResult::new(self.transport().clone(), id_future)
    }
}
