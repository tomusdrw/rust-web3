//! `Eth` namespace, subscriptions

use crate::{
    api::Namespace,
    error, helpers,
    types::{BlockHeader, Filter, Log, SyncState, H256},
    DuplexTransport,
};
use futures::{
    task::{Context, Poll},
    Stream,
};
use pin_project::{pin_project, pinned_drop};
use std::{marker::PhantomData, pin::Pin};

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
#[pin_project(PinnedDrop)]
#[derive(Debug)]
pub struct SubscriptionStream<T: DuplexTransport, I> {
    transport: T,
    id: SubscriptionId,
    #[pin]
    rx: T::NotificationStream,
    _marker: PhantomData<I>,
}

impl<T: DuplexTransport, I> SubscriptionStream<T, I> {
    fn new(transport: T, id: SubscriptionId) -> error::Result<Self> {
        let rx = transport.subscribe(id.clone())?;
        Ok(SubscriptionStream {
            transport,
            id,
            rx,
            _marker: PhantomData,
        })
    }

    /// Return the ID of this subscription
    pub fn id(&self) -> &SubscriptionId {
        &self.id
    }

    /// Unsubscribe from the event represented by this stream
    pub async fn unsubscribe(self) -> error::Result<bool> {
        let &SubscriptionId(ref id) = &self.id;
        let id = helpers::serialize(&id);
        let response = self.transport.execute("eth_unsubscribe", vec![id]).await?;
        helpers::decode(response)
    }
}

impl<T, I> Stream for SubscriptionStream<T, I>
where
    T: DuplexTransport,
    I: serde::de::DeserializeOwned,
{
    type Item = error::Result<I>;

    fn poll_next(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let x = ready!(this.rx.poll_next(ctx));
        Poll::Ready(x.map(|result| serde_json::from_value(result).map_err(Into::into)))
    }
}

#[pinned_drop]
impl<T, I> PinnedDrop for SubscriptionStream<T, I>
where
    T: DuplexTransport,
{
    fn drop(self: Pin<&mut Self>) {
        let _ = self.transport.unsubscribe(self.id().clone());
    }
}

impl<T: DuplexTransport> EthSubscribe<T> {
    /// Create a new heads subscription
    pub async fn subscribe_new_heads(&self) -> error::Result<SubscriptionStream<T, BlockHeader>> {
        let subscription = helpers::serialize(&&"newHeads");
        let response = self.transport.execute("eth_subscribe", vec![subscription]).await?;
        let id: String = helpers::decode(response)?;
        SubscriptionStream::new(self.transport.clone(), SubscriptionId(id))
    }

    /// Create a logs subscription
    pub async fn subscribe_logs(&self, filter: Filter) -> error::Result<SubscriptionStream<T, Log>> {
        let subscription = helpers::serialize(&&"logs");
        let filter = helpers::serialize(&filter);
        let response = self
            .transport
            .execute("eth_subscribe", vec![subscription, filter])
            .await?;
        let id: String = helpers::decode(response)?;
        SubscriptionStream::new(self.transport.clone(), SubscriptionId(id))
    }

    /// Create a pending transactions subscription
    pub async fn subscribe_new_pending_transactions(&self) -> error::Result<SubscriptionStream<T, H256>> {
        let subscription = helpers::serialize(&&"newPendingTransactions");
        let response = self.transport.execute("eth_subscribe", vec![subscription]).await?;
        let id: String = helpers::decode(response)?;
        SubscriptionStream::new(self.transport.clone(), SubscriptionId(id))
    }

    /// Create a sync status subscription
    pub async fn subscribe_syncing(&self) -> error::Result<SubscriptionStream<T, SyncState>> {
        let subscription = helpers::serialize(&&"syncing");
        let response = self.transport.execute("eth_subscribe", vec![subscription]).await?;
        let id: String = helpers::decode(response)?;
        SubscriptionStream::new(self.transport.clone(), SubscriptionId(id))
    }
}
