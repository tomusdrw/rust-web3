//! `Web3` namespace

use api::Namespace;
use helpers::{self, CallResult};
use types::{Bytes, H256};

use {Transport};

/// `Web3` namespace
pub struct Web3<T> {
  transport: T,
}

impl<T: Transport> Namespace<T> for Web3<T> {
  fn new(transport: T) -> Self where Self: Sized {
    Web3 {
      transport: transport,
    }
  }
}

impl<T: Transport> Web3<T> {
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
    Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()) => 0x123
  );
}
