//! Ethereum JSON-RPC client (Web3).

#![warn(missing_docs)]

extern crate futures;
extern crate jsonrpc_core as rpc;
extern crate rustc_serialize;
extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate log;

#[macro_use]
mod helpers;

pub mod api;
mod types;
pub mod transports;

pub use api::Web3Main as Web3;

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
  type Out: futures::Future<Item=rpc::Value, Error=Error> + Send + 'static;

  /// Execute remote method with given parameters.
  fn execute(&self, method: &str, params: Vec<String>) -> Self::Out;
}
