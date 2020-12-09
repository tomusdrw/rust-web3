//! `Web3` namespace

use crate::api::Namespace;
use crate::helpers::{self, CallFuture};
use crate::types::{Bytes, H256};

use crate::Transport;

/// `Web3` namespace
#[derive(Debug, Clone)]
pub struct Web3<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for Web3<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        Web3 { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Web3<T> {
    /// Returns client version
    pub fn client_version(&self) -> CallFuture<String, T::Out> {
        CallFuture::new(self.transport.execute("web3_clientVersion", vec![]))
    }

    /// Returns sha3 of the given data
    pub fn sha3(&self, bytes: Bytes) -> CallFuture<H256, T::Out> {
        let bytes = helpers::serialize(&bytes);
        CallFuture::new(self.transport.execute("web3_sha3", vec![bytes]))
    }
}

#[cfg(test)]
mod tests {
    use crate::api::Namespace;
    use crate::rpc::Value;
    use crate::types::H256;

    use super::Web3;
    use hex_literal::hex;

    rpc_test! (
      Web3:client_version => "web3_clientVersion";
      Value::String("Test123".into()) => "Test123"
    );

    rpc_test! (
      Web3:sha3, hex!("01020304")
      =>
      "web3_sha3", vec![r#""0x01020304""#];
      Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()) => H256::from_low_u64_be(0x123)
    );
}
