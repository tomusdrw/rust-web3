//! `Web3` namespace

use futures::Future;

use api::Namespace;
use helpers;
use types::{Bytes, H256};

use {Result, Transport};

/// List of methods from `web3` namespace
pub trait Web3 {
  /// Returns client version
  fn client_version(&self) -> Result<String>;

  /// Returns sha3 of the given data
  fn sha3(&self, bytes: Bytes) -> Result<H256>;
}

/// `Web3Api` namespace
pub struct Web3Api<'a, T: 'a> {
  transport: &'a T,
}

impl<'a, T: Transport + 'a> Namespace<'a, T> for Web3Api<'a, T> {
  fn new(transport: &'a T) -> Self where Self: Sized {
    Web3Api {
      transport: transport,
    }
  }
}

impl<'a, T: Transport + 'a> Web3 for Web3Api<'a, T> {
  fn client_version(&self) -> Result<String> {
    self.transport.execute("web3_clientVersion", vec![])
      .and_then(helpers::to_string)
      .boxed()
  }

  fn sha3(&self, bytes: Bytes) -> Result<H256> {
    let bytes = helpers::serialize(&bytes);
    self.transport.execute("web3_sha3", vec![bytes])
      .and_then(helpers::to_h256)
      .boxed()
  }
}

#[cfg(test)]
mod tests {
  use futures::Future;

  use api::Namespace;
  use types::{Bytes};
  use rpc::Value;

  use super::{Web3, Web3Api};

  rpc_test! (
    Web3Api:client_version => "web3_clientVersion";
    Value::String("Test123".into()) => "Test123"
  );

  rpc_test! (
    Web3Api:sha3, Bytes(vec![1, 2, 3, 4])
    =>
    "web3_sha3", vec![r#""0x01020304""#];
    Value::String("0x123".into()) => "0x123"
  );
}
