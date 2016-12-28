//! `Net` namespace

use futures::Future;

use api::Namespace;
use helpers;

use {Result, Transport};

/// List of methods from `net` namespace
pub trait NetApi {
  /// Returns protocol version
  fn version(&self) -> Result<String>;

  /// Returns numbers of peers connected to node.
  fn peer_count(&self) -> Result<String>;

  /// Returns true if client is actively listening for network connections.
  fn is_listening(&self) -> Result<bool>;
}

/// `Net` namespace
pub struct Net<'a, T: 'a> {
  transport: &'a T,
}

impl<'a, T: Transport + 'a> Namespace<'a, T> for Net<'a, T> {
  fn new(transport: &'a T) -> Self where Self: Sized {
    Net {
      transport: transport,
    }
  }
}

impl<'a, T: Transport + 'a> NetApi for Net<'a, T> {
  fn version(&self) -> Result<String> {
    self.transport.execute("net_version", vec![])
      .and_then(helpers::to_string)
      .boxed()
  }

  fn peer_count(&self) -> Result<String> {
    self.transport.execute("net_peerCount", vec![])
      .and_then(helpers::to_string)
      .boxed()
  }

  fn is_listening(&self) -> Result<bool> {
    self.transport.execute("net_listening", vec![])
      .and_then(helpers::to_bool)
      .boxed()
  }
}

#[cfg(test)]
mod tests {
  use futures::Future;

  use api::Namespace;
  use rpc::Value;

  use super::{Net, NetApi};

  rpc_test! (
    Net:version => "net_version";
    Value::String("Test123".into()) => "Test123"
  );

  rpc_test! (
    Net:peer_count => "net_peerCount";
    Value::String("Test123".into()) => "Test123"
  );

  rpc_test! (
    Net:is_listening => "net_listening";
    Value::Bool(true) => true
  );
}
