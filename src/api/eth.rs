//! `Eth` namespace

use futures::Future;

use helpers;
use types::{Address, BlockNumber, U256, Bytes, BlockId, TransactionRequest};
use {Result, Transport};

/// List of methods from `eth` namespace
pub trait EthApi {
  /// Get list of available accounts.
  fn accounts(&self) -> Result<Vec<Address>>;

  /// Get current block number
  fn block_number(&self) -> Result<BlockNumber>;

  /// Call a contract without changing the state of the blockchain.
  fn call(&self, TransactionRequest, Option<BlockId>) -> Result<Bytes>;

  /// Get coinbase address
  fn coinbase(&self) -> Result<Address>;

  /// Get current recommended gas price
  fn gas_price(&self) -> Result<U256>;

  /// Get supported compilers
  fn compilers(&self) -> Result<Vec<String>>;

  // TODO [ToDr] Proper types
  /// Get work package
  fn work(&self) -> Result<()>;

  // TODO [ToDr] Proper types
  /// Get hash rate
  fn hashrate(&self) -> Result<()>;

  /// Get mining status
  fn mining(&self) -> Result<bool>;

  /// Start new block filter
  fn new_block_filter(&self) -> Result<U256>;

  /// Start new pending transaction filter
  fn new_pending_transaction_filter(&self) -> Result<U256>;

  /// Start new pending transaction filter
  fn protocol_version(&self) -> Result<String>;

  /// Get syncing status
  fn syncing(&self) -> Result<bool>;
}

/// `Eth` namespace
pub struct Eth<'a, T: 'a> {
  transport: &'a T,
}

impl<'a, T: Transport + 'a> Eth<'a, T> {
  /// New `Eth` namespace with given transport.
  pub fn new(transport: &'a T) -> Self {
    Eth {
      transport: transport,
    }
  }
}

impl<'a, T: Transport + 'a> EthApi for Eth<'a, T> {
  fn accounts(&self) -> Result<Vec<Address>> {
    self.transport.execute("eth_accounts", None)
      .and_then(helpers::to_vector)
      .boxed()
  }

  fn block_number(&self) -> Result<BlockNumber> {
    self.transport.execute("eth_blockNumber", None)
      .and_then(helpers::to_string)
      .boxed()
  }

  fn call(&self, req: TransactionRequest, block: Option<BlockId>) -> Result<Bytes> {
    let req = helpers::serialize(&req);
    let block = helpers::serialize(&block.unwrap_or(BlockId::Latest));

    self.transport.execute("eth_call", Some(vec![req, block]))
      .and_then(helpers::to_bytes)
      .boxed()
  }

  fn coinbase(&self) -> Result<Address> {
    self.transport.execute("eth_coinbase", None)
      .and_then(helpers::to_string)
      .boxed()
  }

  fn gas_price(&self) -> Result<U256> {
    self.transport.execute("eth_gasPrice", None)
      .and_then(helpers::to_string)
      .boxed()
  }

  fn compilers(&self) -> Result<Vec<String>> {
    self.transport.execute("eth_getCompilers", None)
      .and_then(helpers::to_vector)
      .boxed()
  }

  fn work(&self) -> Result<()> {
    self.transport.execute("eth_getWork", None)
      .and_then(|_| Ok(()))
      .boxed()
  }

  fn hashrate(&self) -> Result<()> {
    self.transport.execute("eth_hashrate", None)
      .and_then(|_| Ok(()))
      .boxed()
  }

  fn mining(&self) -> Result<bool> {
    self.transport.execute("eth_mining", None)
      .and_then(helpers::to_bool)
      .boxed()
  }

  fn new_block_filter(&self) -> Result<U256> {
    self.transport.execute("eth_newBlockFilter", None)
      .and_then(helpers::to_string)
      .boxed()
  }

  fn new_pending_transaction_filter(&self) -> Result<U256> {
    self.transport.execute("eth_newPendingTransactionFilter", None)
      .and_then(helpers::to_string)
      .boxed()
  }

  fn protocol_version(&self) -> Result<String> {
    self.transport.execute("eth_protocolVersion", None)
      .and_then(helpers::to_string)
      .boxed()
  }

  fn syncing(&self) -> Result<bool> {
    self.transport.execute("eth_syncing", None)
      .and_then(helpers::to_bool)
      .boxed()
  }
}

#[cfg(test)]
mod tests {
  use futures::Future;
  use types::{TransactionRequest};
  use {Error};

  use super::{Eth, EthApi};

  // TODO [ToDr] Validate deserialization!
  
  rpc_test! (Eth:accounts => "eth_accounts");
  rpc_test! (Eth:block_number => "eth_blockNumber");

  rpc_test! (
    Eth:call, TransactionRequest {
      from: None, to: "0x123".into(),
      gas: None, gas_price: None,
      value: Some("0x1".into()), data: None,
    }, None 
    =>
    "eth_call", vec![r#"{"to":"0x123","value":"0x1"}"#, r#""latest""#]
  );

  rpc_test! (Eth:coinbase => "eth_coinbase");
  // eth_compile*
  // eth_estimateGas
  rpc_test! (Eth:gas_price => "eth_gasPrice");
  // eth_getBalance
  // eth_getBlock*
  // eth_getCode
  rpc_test! (Eth:compilers => "eth_getCompilers");
  // eth_getFilterChanges
  // eth_getFilterChangesEx
  // eth_getFilterLogs
  // eth_getLogs
  // eth_getStorageAt
  // eth_getTransaction*
  // eth_getUncle*
  rpc_test! (Eth:work => "eth_getWork");
  rpc_test! (Eth:hashrate => "eth_hashrate");
  rpc_test! (Eth:mining => "eth_mining");
  rpc_test! (Eth:new_block_filter => "eth_newBlockFilter");
  // eth_newFilter
  // eth_newFilterEx
  rpc_test! (Eth:new_pending_transaction_filter => "eth_newPendingTransactionFilter");
  rpc_test! (Eth:protocol_version => "eth_protocolVersion");
  // eth_sendRawTransaction
  // eth_sendTransaction
  // eth_sign
  // eth_signTransaction
  // eth_submitHashrate
  // eth_submitWork
  rpc_test! (Eth:syncing => "eth_syncing");
  // eth_uninstallFilter
}
