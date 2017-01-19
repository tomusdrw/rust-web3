//! Ethereum JSON-RPC client (Web3).

#![warn(missing_docs)]

extern crate ethabi;
extern crate jsonrpc_core as rpc;
extern crate rustc_serialize;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate log;

/// Re-export of the `futures` crate.
#[macro_use]
pub extern crate futures;

#[macro_use]
mod helpers;

use futures::Future;

pub mod api;
pub mod contract;
pub mod transports;
pub mod types;

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

/// Transport implementation
pub trait Transport {
  /// The type of future this transport returns when a call is made.
  type Out: futures::Future<Item=rpc::Value, Error=Error>;

  /// Execute remote method with given parameters.
  fn execute(&self, method: &str, params: Vec<rpc::Value>) -> Self::Out;

  /// Erase the type of the transport by boxing it and boxing all produced
  /// futures.
  fn erase(self) -> Erased where Self: Sized + 'static, Self::Out: Send + 'static {
    Erased(Box::new(Eraser(self)))
  }
}

// Transport eraser.
struct Eraser<T: Transport>(T);

impl<T: Transport> Transport for Eraser<T>
  where T::Out: Send + 'static
{
  type Out = Result<rpc::Value>;

  fn execute(&self, method: &str, params: Vec<rpc::Value>) -> Self::Out {
    self.0.execute(method, params).boxed()
  }
}

/// Transport with erased output type.
pub struct Erased(Box<Transport<Out=Result<rpc::Value>>>);

impl Transport for Erased {
  type Out = Result<rpc::Value>;

  fn execute(&self, method: &str, params: Vec<rpc::Value>) -> Self::Out {
    self.0.execute(method, params)
  }
}

impl<'a, T: 'a + ?Sized> Transport for &'a T where T: Transport {
  type Out = T::Out;

  fn execute(&self, method: &str, params: Vec<rpc::Value>) -> T::Out {
    (&**self).execute(method, params)
  }
}
