//! A strongly-typed transport alternative.

use crate::{api, error, rpc, BatchTransport, DuplexTransport, RequestId, Transport};

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
    AOut: futures::Future<Output = error::Result<rpc::Value>> + Unpin + 'static,
    BOut: futures::Future<Output = error::Result<rpc::Value>> + Unpin + 'static,
{
    type Out = Box<dyn futures::Future<Output = error::Result<rpc::Value>> + Unpin>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        match *self {
            Self::Left(ref a) => a.prepare(method, params),
            Self::Right(ref b) => b.prepare(method, params),
        }
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        match *self {
            Self::Left(ref a) => Box::new(a.send(id, request)),
            Self::Right(ref b) => Box::new(b.send(id, request)),
        }
    }
}

impl<A, B, ABatch, BBatch> BatchTransport for Either<A, B>
where
    A: BatchTransport<Batch = ABatch>,
    B: BatchTransport<Batch = BBatch>,
    A::Out: Unpin + 'static,
    B::Out: Unpin + 'static,
    ABatch: futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + Unpin + 'static,
    BBatch: futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + Unpin + 'static,
{
    type Batch = Box<dyn futures::Future<Output = error::Result<Vec<error::Result<rpc::Value>>>> + Unpin>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)>,
    {
        match *self {
            Self::Left(ref a) => Box::new(a.send_batch(requests)),
            Self::Right(ref b) => Box::new(b.send_batch(requests)),
        }
    }
}

impl<A, B, AStream, BStream> DuplexTransport for Either<A, B>
where
    A: DuplexTransport<NotificationStream = AStream>,
    B: DuplexTransport<NotificationStream = BStream>,
    A::Out: Unpin + 'static,
    B::Out: Unpin + 'static,
    AStream: futures::Stream<Item = rpc::Value> + Unpin + 'static,
    BStream: futures::Stream<Item = rpc::Value> + Unpin + 'static,
{
    type NotificationStream = Box<dyn futures::Stream<Item = rpc::Value> + Unpin>;

    fn subscribe(&self, id: api::SubscriptionId) -> error::Result<Self::NotificationStream> {
        Ok(match *self {
            Self::Left(ref a) => Box::new(a.subscribe(id)?),
            Self::Right(ref b) => Box::new(b.subscribe(id)?),
        })
    }

    fn unsubscribe(&self, id: api::SubscriptionId) -> error::Result {
        match *self {
            Self::Left(ref a) => a.unsubscribe(id),
            Self::Right(ref b) => b.unsubscribe(id),
        }
    }
}
