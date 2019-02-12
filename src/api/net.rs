//! `Net` namespace

use api::Namespace;
use helpers::CallFuture;
use types::{U256, PeerType, NodeType};

use Transport;

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

    /// Returns list of connected nodes
    pub fn peer_list(&self, _n: NodeType) -> CallFuture<PeerType, T::Out>{
        match _n {
            NodeType::Parity => CallFuture::new(self.transport.execute("parity_netPeers", vec![])),
            NodeType::Geth => CallFuture::new(self.transport.execute("admin_peers", vec![])),
        }
    }

    /// Whether the node is listening for network connections
    pub fn is_listening(&self) -> CallFuture<bool, T::Out> {
        CallFuture::new(self.transport.execute("net_listening", vec![]))
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;

    use api::Namespace;
    use rpc::Value;
    use types::U256;

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
