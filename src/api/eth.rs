//! `Eth` namespace

use futures::Future;

use helpers;
use types::{Address, BlockNumber, U256};
use {Result, Transport};

/// List of methods from `eth` namespace
pub trait EthApi {
  /// Get list of available accounts.
  fn accounts(&self) -> Result<Vec<Address>>;

  /// Get current block number
  fn block_number(&self) -> Result<BlockNumber>;

  /// Get coinbase address
  fn coinbase(&self) -> Result<Address>;

  /// Get current recommended gas price
  fn gas_price(&self) -> Result<U256>;

  /// Get supported compilers
  fn compilers(&self) -> Result<Vec<String>>;

  /// Get work package
  fn work(&self) -> Result<()>;

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
    unimplemented!()
  }

  fn coinbase(&self) -> Result<Address> { unimplemented!() }

  fn gas_price(&self) -> Result<U256> { unimplemented!() }

  fn compilers(&self) -> Result<Vec<String>> { unimplemented!() }

  fn work(&self) -> Result<()> { unimplemented!() }

  fn hashrate(&self) -> Result<()> { unimplemented!() }

  fn mining(&self) -> Result<bool> { unimplemented!() }

  fn new_block_filter(&self) -> Result<U256> { unimplemented!() }

  fn new_pending_transaction_filter(&self) -> Result<U256> { unimplemented!() }

  fn protocol_version(&self) -> Result<String> { unimplemented!() }

  fn syncing(&self) -> Result<bool> { unimplemented!() }
}

#[cfg(test)]
mod tests {
  use futures::Future;
  use {Error};

  use super::{Eth, EthApi};

  rpc_test_wo_params! (Eth:accounts => "eth_accounts");
  rpc_test_wo_params! (Eth:block_number => "eth_blockNumber");
  // eth_call
  rpc_test_wo_params! (Eth:coinbase => "eth_coinbase");
  // eth_compile*
  // eth_estimateGas
  rpc_test_wo_params! (Eth:gas_price => "eth_gasPrice");
  // eth_getBalance
  // eth_getBlock*
  // eth_getCode
  rpc_test_wo_params! (Eth:compilers => "eth_getCompilers");
  // eth_getFilterChanges
  // eth_getFilterChangesEx
  // eth_getFilterLogs
  // eth_getLogs
  // eth_getStorageAt
  // eth_getTransaction*
  // eth_getUncle*
  rpc_test_wo_params! (Eth:work => "eth_getWork");
  rpc_test_wo_params! (Eth:hashrate => "eth_hashrate");
  rpc_test_wo_params! (Eth:mining => "eth_mining");
  rpc_test_wo_params! (Eth:new_block_filter => "eth_newBlockFilter");
  // eth_newFilter
  // eth_newFilterEx
  rpc_test_wo_params! (Eth:new_pending_transaction_filter => "eth_newPendingTransactionFilter");
  rpc_test_wo_params! (Eth:protocol_version => "eth_protocolVersion");
  // eth_sendRawTransaction
  // eth_sendTransaction
  // eth_sign
  // eth_signTransaction
  // eth_submitHashrate
  // eth_submitWork
  rpc_test_wo_params! (Eth:syncing => "eth_syncing");
  // eth_uninstallFilter
}
