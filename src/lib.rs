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
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
  /// Server is unreachable
  Unreachable,
  /// Unexpected response was returned
  InvalidResponse(String),
  /// Transport Error
  Transport(String),
  /// Error returned by RPC
  Rpc(rpc::Error),
}

/// Transport implementation
pub trait Transport {
  /// Execute remote method with given parameters.
  fn execute(&self, method: &str, params: Vec<String>) -> Result<rpc::Value>;
}

impl<'a, T: 'a + ?Sized> Transport for &'a T where T: Transport {
  fn execute(&self, method: &str, params: Vec<String>) -> Result<rpc::Value> {
    (&**self).execute(method, params)
  }
}
