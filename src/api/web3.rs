//! `Web3` namespace

use api::Namespace;
use helpers::{self, CallResult};
use types::{Bytes, H256};

use {Transport};

/// `Web3` namespace
pub struct Web3<'a, T: 'a> {
  transport: &'a T,
}

impl<'a, T: Transport + 'a> Namespace<'a, T> for Web3<'a, T> {
  fn new(transport: &'a T) -> Self where Self: Sized {
    Web3 {
      transport: transport,
    }
  }
}

impl<'a, T: Transport + 'a> Web3<'a, T> {
  /// Returns client version
  pub fn client_version(&self) -> CallResult<String, T::Out> {
    CallResult::new(self.transport.execute("web3_clientVersion", vec![]))
  }

  /// Returns sha3 of the given data
  pub fn sha3(&self, bytes: Bytes) -> CallResult<H256, T::Out> {
    let bytes = helpers::serialize(&bytes);
    CallResult::new(self.transport.execute("web3_sha3", vec![bytes]))
  }
}

#[cfg(test)]
mod tests {
  use futures::Future;

  use api::Namespace;
  use types::{Bytes};
  use rpc::Value;

  use super::Web3;

  rpc_test! (
    Web3:client_version => "web3_clientVersion";
    Value::String("Test123".into()) => "Test123"
  );

  rpc_test! (
    Web3:sha3, Bytes(vec![1, 2, 3, 4])
    =>
    "web3_sha3", vec![r#""0x01020304""#];
    Value::String("0x123".into()) => "0x123"
  );
}
