//! A strongly-typed transport alternative.

use crate::{api, error, rpc, BatchTransport, DuplexTransport, RequestId, Transport};
use futures::{
    future::{BoxFuture, FutureExt},
    stream::{BoxStream, StreamExt},
};

/// A wrapper over two possible transports.
///
/// This type can be used to write semi-generic
/// code without the hassle of making all functions generic.
///
/// See the `examples` folder for an example how to use it.
#[derive(Debug, Clone)]
pub enum Either<A, B> {
    /// First possible transport.
    Left(A),
    /// Second possible transport.
    Right(B),
}

impl<A, B, AOut, BOut> Transport for Either<A, B>
where
    A: Transport<Out = AOut>,
    B: Transport<Out = BOut>,
    AOut: futures::Future<Output = error::Result<rpc::Value>> + 'static + Send,
    BOut: futures::Future<Output = error::Result<rpc::Value>> + 'static + Send,
{
    type Out = BoxFuture<'static, error::Result<rpc::Value>>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        match *self {
            Self::Left(ref a) => a.prepare(method, params),
            Self::Right(ref b) => b.prepare(method, params),
        }
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        match *self {
            Self::Left(ref a) => a.send(id, request).boxed(),
            Self::Right(ref b) => b.send(id, request).boxed(),
        }
    }
}

impl<A, B, ABatch, BBatch> BatchTransport for Either<A, B>
where
    A: BatchTransport<Batch = ABatch>,
    B: BatchTransport<Batch = BBatch>,
    A::Out: 'static + Send,
    B::Out: 'static + Send,
    ABatch: futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + 'static + Send,
    BBatch: futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + 'static + Send,
{
    type Batch = BoxFuture<'static, error::Result<Vec<error::Result<rpc::Value>>>>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)>,
    {
        match *self {
            Self::Left(ref a) => a.send_batch(requests).boxed(),
            Self::Right(ref b) => b.send_batch(requests).boxed(),
        }
    }
}

impl<A, B, AStream, BStream> DuplexTransport for Either<A, B>
where
    A: DuplexTransport<NotificationStream = AStream>,
    B: DuplexTransport<NotificationStream = BStream>,
    A::Out: 'static + Send,
    B::Out: 'static + Send,
    AStream: futures::Stream<Item = rpc::Value> + 'static + Send,
    BStream: futures::Stream<Item = rpc::Value> + 'static + Send,
{
    type NotificationStream = BoxStream<'static, rpc::Value>;

    fn subscribe(&self, id: api::SubscriptionId) -> error::Result<Self::NotificationStream> {
        Ok(match *self {
            Self::Left(ref a) => a.subscribe(id)?.boxed(),
            Self::Right(ref b) => b.subscribe(id)?.boxed(),
        })
    }

    fn unsubscribe(&self, id: api::SubscriptionId) -> error::Result {
        match *self {
            Self::Left(ref a) => a.unsubscribe(id),
            Self::Right(ref b) => b.unsubscribe(id),
        }
    }
}
