//! Ethereum JSON-RPC client (Web3).

#![warn(missing_docs)]

extern crate ethabi;
extern crate jsonrpc_core as rpc;
extern crate parking_lot;
extern crate rustc_serialize;
extern crate serde;
extern crate tokio_timer;

#[cfg_attr(test, macro_use)]
extern crate serde_json;

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

/// Re-export of the `futures` crate.
#[macro_use]
pub extern crate futures;

// it needs to be before other modules
// otherwise the macro for tests is not available.
#[macro_use]
pub mod helpers;

pub mod api;
pub mod contract;
pub mod transports;
pub mod types;

pub mod confirm;

use futures::Future;

pub use api::{Web3Main as Web3, ErasedWeb3};

/// RPC result
pub type Result<T> = futures::BoxFuture<T, Error>;

/// RPC error
#[derive(Debug, Clone, PartialEq)]
pub enum Error {
  /// Server is unreachable
  Unreachable,
  /// Unexpected response was returned
  InvalidResponse(String),
  /// Transport Error
  Transport(String),
  /// JSON decoding error.
  Decoder(String),
  /// Error returned by RPC
  Rpc(rpc::Error),
  /// Internal Error
  Internal,
}

impl From<serde_json::Error> for Error {
  fn from(err: serde_json::Error) -> Self {
    Error::Decoder(format!("{:?}", err))
  }
}

impl From<rpc::Error> for Error {
  fn from(err: rpc::Error) -> Self {
    Error::Rpc(err)
  }
}

/// Assigned RequestId
pub type RequestId = usize;

/// Transport implementation
pub trait Transport {
  /// The type of future this transport returns when a call is made.
  type Out: futures::Future<Item=rpc::Value, Error=Error>;

  /// Prepare serializable RPC call for given method with parameters.
  fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call);

  /// Execute prepared RPC call.
  fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out;

  /// Execute remote method with given parameters.
  fn execute(&self, method: &str, params: Vec<rpc::Value>) -> Self::Out {
    let (id, request) = self.prepare(method, params);
    self.send(id, request)
  }

  /// Erase the type of the transport by boxing it and boxing all produced
  /// futures.
  fn erase(self) -> Erased where
    Self: Sized + 'static,
    Self::Out: Send + 'static,
  {
    Erased(Box::new(Eraser(self)))
  }
}

/// A transport implementation supporting batch requests.
pub trait BatchTransport: Transport {
  /// The type of future this transport returns when a call is made.
  type Batch: futures::Future<Item=Vec<::std::result::Result<rpc::Value, Error>>, Error=Error>;

  /// Sends a batch of prepared RPC calls.
  fn send_batch<T>(&self, requests: T) -> Self::Batch where
    T: IntoIterator<Item=(RequestId, rpc::Call)>;
}

/// Transport eraser.
struct Eraser<T>(T);

impl<T: Transport> Transport for Eraser<T> where
  T::Out: Send + 'static,
{
  type Out = Result<rpc::Value>;

  fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
    self.0.prepare(method, params)
  }

  fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
    self.0.send(id, request).boxed()
  }
}

/// Transport with erased output type.
pub struct Erased(Box<Transport<Out=Result<rpc::Value>>>);

impl Transport for Erased {
  type Out = Result<rpc::Value>;

  fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
    self.0.prepare(method, params)
  }

  fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
    self.0.send(id, request)
  }
}

impl<X, T> Transport for X where
  T: Transport + ?Sized,
  X: ::std::ops::Deref<Target=T>,
{
  type Out = T::Out;

  fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
    (**self).prepare(method, params)
  }

  fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
    (**self).send(id, request)
  }
}

impl<X, T> BatchTransport for X where
  T: BatchTransport + ?Sized,
  X: ::std::ops::Deref<Target=T>,
{
  type Batch = T::Batch;

  fn send_batch<I>(&self, requests: I) -> Self::Batch where
    I: IntoIterator<Item=(RequestId, rpc::Call)>
  {
    (**self).send_batch(requests)
  }
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;
  use api::Web3Main;
  use futures::BoxFuture;
  use super::{rpc, Error, Transport, RequestId};

  struct FakeTransport;
  impl Transport for FakeTransport {
    type Out = BoxFuture<rpc::Value, Error>;

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

    let _web3_1 = Web3Main::new(transport);
    let _web3_2 = Web3Main::new(transport2);
  }
}
