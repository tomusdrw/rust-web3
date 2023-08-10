//! A strongly-typed transport alternative.

use crate::{api, error, rpc, BatchTransport, DuplexTransport, RequestId, Transport};
#[cfg(feature = "wasm")]
use futures::{future::LocalBoxFuture as BoxFuture, stream::LocalBoxStream as BoxStream};
#[cfg(not(feature = "wasm"))]
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

#[cfg(not(feature = "wasm"))]
trait Out: futures::Future<Output = error::Result<rpc::Value>> + 'static + Send {}
#[cfg(not(feature = "wasm"))]
impl<T> Out for T where T: futures::Future<Output = error::Result<rpc::Value>> + 'static + Send {}
#[cfg(feature = "wasm")]
trait Out: futures::Future<Output = error::Result<rpc::Value>> + 'static {}
#[cfg(feature = "wasm")]
impl<T> Out for T where T: futures::Future<Output = error::Result<rpc::Value>> + 'static {}

impl<A, B, AOut, BOut> Transport for Either<A, B>
where
    A: Transport<Out = AOut>,
    B: Transport<Out = BOut>,
    AOut: Out,
    BOut: Out,
{
    type Out = BoxFuture<'static, error::Result<rpc::Value>>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        match *self {
            Self::Left(ref a) => a.prepare(method, params),
            Self::Right(ref b) => b.prepare(method, params),
        }
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        #[cfg(not(feature = "wasm"))]
        match *self {
            Self::Left(ref a) => a.send(id, request).boxed(),
            Self::Right(ref b) => b.send(id, request).boxed(),
        }
        #[cfg(feature = "wasm")]
        match *self {
            Self::Left(ref a) => Box::pin(a.send(id, request)),
            Self::Right(ref b) => Box::pin(b.send(id, request)),
        }
    }
}

#[cfg(not(feature = "wasm"))]
trait Batch: futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + 'static + Send {}
#[cfg(not(feature = "wasm"))]
impl<T> Batch for T where T: futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + 'static + Send {}
#[cfg(feature = "wasm")]
trait Batch: futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + 'static {}
#[cfg(feature = "wasm")]
impl<T> Batch for T where T: futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + 'static {}

impl<A, B, ABatch, BBatch> BatchTransport for Either<A, B>
where
    A: BatchTransport<Batch = ABatch>,
    B: BatchTransport<Batch = BBatch>,
    A::Out: Out,
    B::Out: Out,
    ABatch: Batch,
    BBatch: Batch,
{
    type Batch = BoxFuture<'static, error::Result<Vec<error::Result<rpc::Value>>>>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)>,
    {
        #[cfg(not(feature = "wasm"))]
        match *self {
            Self::Left(ref a) => a.send_batch(requests).boxed(),
            Self::Right(ref b) => b.send_batch(requests).boxed(),
        }
        #[cfg(feature = "wasm")]
        match *self {
            Self::Left(ref a) => Box::pin(a.send_batch(requests)),
            Self::Right(ref b) => Box::pin(b.send_batch(requests)),
        }
    }
}

#[cfg(not(feature = "wasm"))]
trait NotificationStream: futures::Stream<Item = rpc::Value> + 'static + Send {}
#[cfg(not(feature = "wasm"))]
impl<T> NotificationStream for T where T: futures::Stream<Item = rpc::Value> + 'static + Send {}
#[cfg(feature = "wasm")]
trait NotificationStream: futures::Stream<Item = rpc::Value> + 'static {}
#[cfg(feature = "wasm")]
impl<T> NotificationStream for T where T: futures::Stream<Item = rpc::Value> + 'static {}

impl<A, B, AStream, BStream> DuplexTransport for Either<A, B>
where
    A: DuplexTransport<NotificationStream = AStream>,
    B: DuplexTransport<NotificationStream = BStream>,
    A::Out: Out,
    B::Out: Out,
    AStream: NotificationStream,
    BStream: NotificationStream,
{
    type NotificationStream = BoxStream<'static, rpc::Value>;

    fn subscribe(&self, id: api::SubscriptionId) -> error::Result<Self::NotificationStream> {
        Ok({
            #[cfg(not(feature = "wasm"))]
            match *self {
                Self::Left(ref a) => a.subscribe(id)?.boxed(),
                Self::Right(ref b) => b.subscribe(id)?.boxed(),
            }
            #[cfg(feature = "wasm")]
            match *self {
                Self::Left(ref a) => Box::pin(a.subscribe(id)?),
                Self::Right(ref b) => Box::pin(b.subscribe(id)?),
            }
        })
    }

    fn unsubscribe(&self, id: api::SubscriptionId) -> error::Result {
        match *self {
            Self::Left(ref a) => a.unsubscribe(id),
            Self::Right(ref b) => b.unsubscribe(id),
        }
    }
}
