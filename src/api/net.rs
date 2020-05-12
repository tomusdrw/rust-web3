//! `Net` namespace

use crate::api::Namespace;
use crate::helpers::CallFuture;
use crate::types::U256;

use crate::Transport;

/// `Net` namespace
#[derive(Debug, Clone)]
pub struct Net<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for Net<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        Net { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Net<T> {
    /// Returns protocol version
    pub fn version(&self) -> CallFuture<String, T::Out> {
        CallFuture::new(self.transport.execute("net_version", vec![]))
    }

    /// Returns number of peers connected to node.
    pub fn peer_count(&self) -> CallFuture<U256, T::Out> {
        CallFuture::new(self.transport.execute("net_peerCount", vec![]))
    }

    /// Whether the node is listening for network connections
    pub fn is_listening(&self) -> CallFuture<bool, T::Out> {
        CallFuture::new(self.transport.execute("net_listening", vec![]))
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;

    use crate::api::Namespace;
    use crate::rpc::Value;
    use crate::types::U256;

    use super::Net;

    rpc_test! (
      Net:version => "net_version";
      Value::String("Test123".into()) => "Test123"
    );

    rpc_test! (
      Net:peer_count => "net_peerCount";
      Value::String("0x123".into()) => U256::from(0x123)
    );

    rpc_test! (
      Net:is_listening => "net_listening";
      Value::Bool(true) => true
    );
}
