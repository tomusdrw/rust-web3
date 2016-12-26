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

pub use api::Web3;

/// RPC result
pub type Result<T> = futures::BoxFuture<T, Error>;

/// RPC error
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
  /// Server is unreachable
  Unreachable,
  /// Unexpected response was returned
  InvalidResponse(String),
}

/// Transport implementation
pub trait Transport {
  /// Execute remote method with given parameters.
  fn execute(&self, method: &str, params: Option<Vec<String>>) -> Result<rpc::Value>;
}
