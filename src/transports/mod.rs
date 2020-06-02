//! Supported Ethereum JSON-RPC transports.

pub mod batch;
pub use self::batch::Batch;

use super::*;
#[derive(Debug, Clone)]
pub struct Dummy;

impl Dummy {
    /// asd
    pub fn new(_s: &str) -> error::Result<Self> {
        Ok(Dummy)
    }
}

impl Transport for Dummy {
    type Out = futures::future::Ready<error::Result<rpc::Value>>;

    fn prepare(&self, _method: &str, _params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        unimplemented!()
    }

    fn send(&self, _id: RequestId, _request: rpc::Call) -> Self::Out {
        unimplemented!()
    }
}

impl BatchTransport for Dummy {
    /// The type of future this transport returns when a call is made.
    type Batch = futures::future::Ready<error::Result<Vec<error::Result<rpc::Value>>>>;

    /// Sends a batch of prepared RPC calls.
    fn send_batch<T>(&self, _requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)> {
        unimplemented!()
    }
}

impl DuplexTransport for Dummy {
    type NotificationStream = futures::stream::Iter<
        std::vec::IntoIter<error::Result<rpc::Value>>
        >;

    fn subscribe(&self, _id: &api::SubscriptionId) -> Self::NotificationStream {
        unimplemented!()
    }

    fn unsubscribe(&self, _id: &api::SubscriptionId) {
        unimplemented!()
    }
}

// pub use Dummy as Http;
pub use Dummy as WebSocket;
pub use Dummy as Ipc;

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "http")]
pub use self::http::Http;
//
// #[cfg(feature = "ws")]
// pub mod ws;
// #[cfg(feature = "ws")]
// pub use self::ws::WebSocket;
