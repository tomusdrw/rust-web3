//! `Eth` namespace

use futures::Future;

use helpers;
use types::{Address, U256, Bytes, BlockNumber, BlockId, TransactionRequest};
use {Result, Transport};

/// List of methods from `eth` namespace
pub trait EthApi {
  /// Get list of available accounts.
  fn accounts(&self) -> Result<Vec<Address>>;

  /// Get current block number
  fn block_number(&self) -> Result<U256>;

  /// Call a constant method of contract without changing the state of the blockchain.
  fn call(&self, TransactionRequest, Option<BlockNumber>) -> Result<Bytes>;

  /// Get coinbase address
  fn coinbase(&self) -> Result<Address>;

  /// Compile LLL
  fn compile_lll(&self, String) -> Result<Bytes>;

  /// Compile Solidity
  fn compile_solidity(&self, String) -> Result<Bytes>;

  /// Compile Serpent
  fn compile_serpent(&self, String) -> Result<Bytes>;

  /// Call a contract without changing the state of the blockchain to estimate gas usage.
  fn estimate_gas(&self, TransactionRequest, Option<BlockNumber>) -> Result<U256>;

  /// Get current recommended gas price
  fn gas_price(&self) -> Result<U256>;

  /// Get balance of given address
  fn balance(&self, Address, Option<BlockNumber>) -> Result<U256>;

  // TODO [ToDr] Proper types
  /// Get block details
  fn block(&self, BlockId, bool) -> Result<()>;

  /// Get number of transactions in block
  fn block_transaction_count(&self, BlockId) -> Result<Option<U256>>;

  /// Get code under given address
  fn code(&self, Address, Option<BlockNumber>) -> Result<Bytes>;

  /// Get supported compilers
  fn compilers(&self) -> Result<Vec<String>>;

  // TODO [ToDr] Proper types
  /// Get work package
  fn work(&self) -> Result<()>;

  /// Get hash rate
  fn hashrate(&self) -> Result<U256>;

  /// Get mining status
  fn mining(&self) -> Result<bool>;

  /// Start new block filter
  fn new_block_filter(&self) -> Result<U256>;

  /// Start new pending transaction filter
  fn new_pending_transaction_filter(&self) -> Result<U256>;

  /// Start new pending transaction filter
  fn protocol_version(&self) -> Result<String>;

  // TODO [ToDr] Proper type?
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

  fn block_number(&self) -> Result<U256> {
    self.transport.execute("eth_blockNumber", None)
      .and_then(helpers::to_u256)
      .boxed()
  }

  fn call(&self, req: TransactionRequest, block: Option<BlockNumber>) -> Result<Bytes> {
    let req = helpers::serialize(&req);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

    self.transport.execute("eth_call", Some(vec![req, block]))
      .and_then(helpers::to_bytes)
      .boxed()
  }

  fn coinbase(&self) -> Result<Address> {
    self.transport.execute("eth_coinbase", None)
      .and_then(helpers::to_address)
      .boxed()
  }

  fn compile_lll(&self, code: String) -> Result<Bytes> {
    let code = helpers::serialize(&code);
    self.transport.execute("eth_compileLLL", Some(vec![code]))
      .and_then(helpers::to_bytes)
      .boxed()
  }

  fn compile_solidity(&self, code: String) -> Result<Bytes> {
    let code = helpers::serialize(&code);
    self.transport.execute("eth_compileSolidity", Some(vec![code]))
      .and_then(helpers::to_bytes)
      .boxed()
  }

  fn compile_serpent(&self, code: String) -> Result<Bytes> {
    let code = helpers::serialize(&code);
    self.transport.execute("eth_compileSerpent", Some(vec![code]))
      .and_then(helpers::to_bytes)
      .boxed()
  }

  fn estimate_gas(&self, req: TransactionRequest, block: Option<BlockNumber>) -> Result<U256> {
    let req = helpers::serialize(&req);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

    self.transport.execute("eth_estimateGas", Some(vec![req, block]))
      .and_then(helpers::to_u256)
      .boxed()
  }

  fn gas_price(&self) -> Result<U256> {
    self.transport.execute("eth_gasPrice", None)
      .and_then(helpers::to_u256)
      .boxed()
  }

  fn balance(&self, address: Address, block: Option<BlockNumber>) -> Result<U256> {
    let address = helpers::serialize(&address);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));
  
    self.transport.execute("eth_getBalance", Some(vec![address, block]))
      .and_then(helpers::to_u256)
      .boxed()
  }

  fn block(&self, block: BlockId, include_txs: bool) -> Result<()> {
    let include_txs = helpers::serialize(&include_txs);

    let result = match block {
      BlockId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getBlockByHash", Some(vec![hash, include_txs]))
      },
      BlockId::Number(num) => {
        let num = helpers::serialize(&num);
        self.transport.execute("eth_getBlockByNumber", Some(vec![num, include_txs]))
      },
    };

    result
      .and_then(|_| Ok(()))
      .boxed()
  }

  fn block_transaction_count(&self, block: BlockId) -> Result<Option<U256>> {
    let result = match block {
      BlockId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getBlockTransactionCountByHash", Some(vec![hash]))
      },
      BlockId::Number(num) => {
        let num = helpers::serialize(&num);
        self.transport.execute("eth_getBlockTransactionCountByNumber", Some(vec![num]))
      },
    };

    result
      .and_then(helpers::to_u256_option)
      .boxed()
  }

  fn code(&self, address: Address, block: Option<BlockNumber>) -> Result<Bytes> {
    let address = helpers::serialize(&address);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));
  
    self.transport.execute("eth_getCode", Some(vec![address, block]))
      .and_then(helpers::to_bytes)
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

  fn hashrate(&self) -> Result<U256> {
    self.transport.execute("eth_hashrate", None)
      .and_then(helpers::to_u256)
      .boxed()
  }

  fn mining(&self) -> Result<bool> {
    self.transport.execute("eth_mining", None)
      .and_then(helpers::to_bool)
      .boxed()
  }

  fn new_block_filter(&self) -> Result<U256> {
    self.transport.execute("eth_newBlockFilter", None)
      .and_then(helpers::to_u256)
      .boxed()
  }

  fn new_pending_transaction_filter(&self) -> Result<U256> {
    self.transport.execute("eth_newPendingTransactionFilter", None)
      .and_then(helpers::to_u256)
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
  use types::{TransactionRequest, Bytes, BlockNumber, BlockId};
  use rpc::Value;

  use super::{Eth, EthApi};

  rpc_test! (
    Eth:accounts => "eth_accounts";
    Value::Array(vec![Value::String("0x123".into())]) => vec!["0x123".into()]
  );

  rpc_test! (
    Eth:block_number => "eth_blockNumber";
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:call, TransactionRequest {
      from: None, to: "0x123".into(),
      gas: None, gas_price: None,
      value: Some("0x1".into()), data: None,
    }, None 
    =>
    "eth_call", vec![r#"{"to":"0x123","value":"0x1"}"#, r#""latest""#];
    Value::String("0x010203".into()) => Bytes(vec![1, 2, 3])
  );

  rpc_test! (
    Eth:coinbase => "eth_coinbase";
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:compile_lll, "code" => "eth_compileLLL", vec![r#""code""#];
    Value::String("0x0123".into()) => Bytes(vec![0x1, 0x23])
  );

  rpc_test! (
    Eth:compile_solidity, "code" => "eth_compileSolidity", vec![r#""code""#];
    Value::String("0x0123".into()) => Bytes(vec![0x1, 0x23])
  );

  rpc_test! (
    Eth:compile_serpent, "code" => "eth_compileSerpent", vec![r#""code""#];
    Value::String("0x0123".into()) => Bytes(vec![0x1, 0x23])
  );

  rpc_test! (
    Eth:estimate_gas, TransactionRequest {
      from: None, to: "0x123".into(),
      gas: None, gas_price: None,
      value: Some("0x1".into()), data: None,
    }, None 
    =>
    "eth_estimateGas", vec![r#"{"to":"0x123","value":"0x1"}"#, r#""latest""#];
    Value::String("0x123".into()) => "0x123"
  );
  
  rpc_test! (
    Eth:gas_price => "eth_gasPrice";
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:balance, "0x123", None 
    =>
    "eth_getBalance", vec![r#""0x123""#, r#""latest""#];
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:block:block_by_hash, BlockId::Hash("0x123".into()), true 
    =>
    "eth_getBlockByHash", vec![r#""0x123""#, r#"true"#];
    Value::Null => ()
  );

  rpc_test! (
    Eth:block, BlockNumber::Pending, true 
    =>
    "eth_getBlockByNumber", vec![r#""pending""#, r#"true"#];
    Value::Null => ()
  );

  rpc_test! (
    Eth:block_transaction_count:block_tx_count_by_hash, "0x123".to_owned()
    =>
    "eth_getBlockTransactionCountByHash", vec![r#""0x123""#];
    Value::String("0x123".into()) => Some("0x123".into())
  );

  rpc_test! (
    Eth:block_transaction_count, BlockNumber::Pending
    =>
    "eth_getBlockTransactionCountByNumber", vec![r#""pending""#];
    Value::Null => None
  );

  rpc_test! (
    Eth:code, "0x123", Some(BlockNumber::Pending)
    =>
    "eth_getCode", vec![r#""0x123""#, r#""pending""#];
    Value::String("0x0123".into()) => Bytes(vec![0x1, 0x23])
  );

  rpc_test! (
    Eth:compilers => "eth_getCompilers";
    Value::Array(vec![]) => vec![]
  );
  // eth_getFilterChanges
  // eth_getFilterChangesEx
  // eth_getFilterLogs
  // eth_getLogs
  // eth_getStorageAt
  // eth_getTransaction*
  // eth_getUncle*
  
  rpc_test! (
    Eth:work => "eth_getWork";
    Value::Null => ()
  );

  rpc_test! (
    Eth:hashrate => "eth_hashrate";
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:mining => "eth_mining";
    Value::Bool(true) => true
  );

  rpc_test! (
    Eth:new_block_filter => "eth_newBlockFilter";
    Value::String("0x123".into()) => "0x123"
  );
  // eth_newFilter
  // eth_newFilterEx
  rpc_test! (
    Eth:new_pending_transaction_filter => "eth_newPendingTransactionFilter";
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:protocol_version => "eth_protocolVersion";
    Value::String("0x123".into()) => "0x123"
  );
  // eth_sendRawTransaction
  // eth_sendTransaction
  // eth_sign
  // eth_signTransaction
  // eth_submitHashrate
  // eth_submitWork
  rpc_test! (
    Eth:syncing => "eth_syncing";
    Value::Bool(true) => true
  );
  // eth_uninstallFilter
}
