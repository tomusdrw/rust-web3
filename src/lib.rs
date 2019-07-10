//! Ethereum JSON-RPC client (Web3).

#![allow(
    clippy::type_complexity,
    clippy::wrong_self_convention,
    clippy::single_match,
    clippy::let_unit_value,
    clippy::match_wild_err_arm
)]
#![warn(missing_docs)]

use jsonrpc_core as rpc;

/// Re-export of the `futures` crate.
#[macro_use]
pub extern crate futures;

// it needs to be before other modules
// otherwise the macro for tests is not available.
#[macro_use]
pub mod helpers;

pub mod api;
pub mod contract;
pub mod error;
pub mod transports;
pub mod types;

pub mod confirm;

pub use crate::api::Web3;
pub use crate::error::Error;

/// RPC result
pub type Result<T> = Box<dyn futures::Future<Item = T, Error = Error> + Send + 'static>;

/// Assigned RequestId
pub type RequestId = usize;

/// Transport implementation
pub trait Transport: ::std::fmt::Debug + Clone {
    /// The type of future this transport returns when a call is made.
    type Out: futures::Future<Item = rpc::Value, Error = Error>;

    /// Prepare serializable RPC call for given method with parameters.
    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call);

    /// Execute prepared RPC call.
    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out;

    /// Execute remote method with given parameters.
    fn execute(&self, method: &str, params: Vec<rpc::Value>) -> Self::Out {
        let (id, request) = self.prepare(method, params);
        self.send(id, request)
    }
}

/// A transport implementation supporting batch requests.
pub trait BatchTransport: Transport {
    /// The type of future this transport returns when a call is made.
    type Batch: futures::Future<Item = Vec<::std::result::Result<rpc::Value, Error>>, Error = Error>;

    /// Sends a batch of prepared RPC calls.
    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)>;
}

/// A transport implementation supporting pub sub subscriptions.
pub trait DuplexTransport: Transport {
    /// The type of stream this transport returns
    type NotificationStream: futures::Stream<Item = rpc::Value, Error = Error>;

    /// Add a subscription to this transport
    fn subscribe(&self, id: &api::SubscriptionId) -> Self::NotificationStream;

    /// Remove a subscription from this transport
    fn unsubscribe(&self, id: &api::SubscriptionId);
}

impl<X, T> Transport for X
where
    T: Transport + ?Sized,
    X: ::std::ops::Deref<Target = T>,
    X: ::std::fmt::Debug,
    X: Clone,
{
    type Out = T::Out;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        (**self).prepare(method, params)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        (**self).send(id, request)
    }
}

impl<X, T> BatchTransport for X
where
    T: BatchTransport + ?Sized,
    X: ::std::ops::Deref<Target = T>,
    X: ::std::fmt::Debug,
    X: Clone,
{
    type Batch = T::Batch;

    fn send_batch<I>(&self, requests: I) -> Self::Batch
    where
        I: IntoIterator<Item = (RequestId, rpc::Call)>,
    {
        (**self).send_batch(requests)
    }
}

impl<X, T> DuplexTransport for X
where
    T: DuplexTransport + ?Sized,
    X: ::std::ops::Deref<Target = T>,
    X: ::std::fmt::Debug,
    X: Clone,
{
    type NotificationStream = T::NotificationStream;

    fn subscribe(&self, id: &api::SubscriptionId) -> Self::NotificationStream {
        (**self).subscribe(id)
    }

    fn unsubscribe(&self, id: &api::SubscriptionId) {
        (**self).unsubscribe(id)
    }
}

#[cfg(test)]
mod tests {
    use super::{rpc, Error, RequestId, Transport};
    use crate::api::Web3;
    use futures::Future;
    use std::sync::Arc;

    #[derive(Debug, Clone)]
    struct FakeTransport;
    impl Transport for FakeTransport {
        type Out = Box<dyn Future<Item = rpc::Value, Error = Error> + Send + 'static>;

        fn prepare(&self, _method: &str, _params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
            unimplemented!()
        }

        fn send(&self, _id: RequestId, _request: rpc::Call) -> Self::Out {
            unimplemented!()
        }
    }

    #[test]
    fn should_allow_to_use_arc_as_transport() {
        let transport = Arc::new(FakeTransport);
        let transport2 = transport.clone();

        let _web3_1 = Web3::new(transport);
        let _web3_2 = Web3::new(transport2);
    }
}
