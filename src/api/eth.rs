//! `Eth` namespace

use futures::Future;

use helpers;
use types::{
  Address, Block, BlockId, BlockNumber, Bytes, CallRequest,
  H64, H256, H512, Index,
  Transaction, TransactionId, TransactionReceipt, TransactionRequest,
  U256, Work,
};
use {Result, Transport};

/// List of methods from `eth` namespace
pub trait EthApi {
  /// Get list of available accounts.
  fn accounts(&self) -> Result<Vec<Address>>;

  /// Get current block number
  fn block_number(&self) -> Result<U256>;

  /// Call a constant method of contract without changing the state of the blockchain.
  fn call(&self, options: CallRequest, block: Option<BlockNumber>) -> Result<Bytes>;

  /// Get coinbase address
  fn coinbase(&self) -> Result<Address>;

  /// Compile LLL
  fn compile_lll(&self, String) -> Result<Bytes>;

  /// Compile Solidity
  fn compile_solidity(&self, String) -> Result<Bytes>;

  /// Compile Serpent
  fn compile_serpent(&self, String) -> Result<Bytes>;

  /// Call a contract without changing the state of the blockchain to estimate gas usage.
  fn estimate_gas(&self, options: CallRequest, block: Option<BlockNumber>) -> Result<U256>;

  /// Get current recommended gas price
  fn gas_price(&self) -> Result<U256>;

  /// Get balance of given address
  fn balance(&self, address: Address, block: Option<BlockNumber>) -> Result<U256>;

  /// Get block details
  fn block(&self, block: BlockId, include_txs: bool) -> Result<Block>;

  /// Get number of transactions in block
  fn block_transaction_count(&self, block: BlockId) -> Result<Option<U256>>;

  /// Get code under given address
  fn code(&self, address: Address, block: Option<BlockNumber>) -> Result<Bytes>;

  /// Get supported compilers
  fn compilers(&self) -> Result<Vec<String>>;

  /// Get storage entry
  fn storage(&self, address: Address, idx: U256, block: Option<BlockNumber>) -> Result<H256>;

  /// Get nonce
  fn transaction_count(&self, address: Address, block: Option<BlockNumber>) -> Result<U256>;

  /// Get transaction
  fn transaction(&self, id: TransactionId) -> Result<Option<Transaction>>;

  /// Get transaction receipt
  fn transaction_receipt(&self, hash: H256) -> Result<Option<TransactionReceipt>>;

  /// Get uncle
  fn uncle(&self, block: BlockId, index: Index) -> Result<Option<Block>>;

  /// Get uncle count in block
  fn uncle_count(&self, block: BlockId) -> Result<Option<U256>>;

  /// Get work package
  fn work(&self) -> Result<Work>;

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

  /// Sends a rlp-encoded signed transaction
  fn send_raw_transaction(&self, rlp: Bytes) -> Result<H256>;

  /// Sends a transaction transaction
  fn send_transaction(&self, tx: TransactionRequest) -> Result<H256>;

  /// Signs a hash of given data
  fn sign(&self, address: Address, data: Bytes) -> Result<H512>;

  /// Submit hashrate of external miner
  fn submit_hashrate(&self, rate: U256, id: H256) -> Result<bool>;

  /// Submit work of external miner
  fn submit_work(&self, nonce: H64, pow_hash: H256, mix_hash: H256) -> Result<bool>;

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

  fn call(&self, req: CallRequest, block: Option<BlockNumber>) -> Result<Bytes> {
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

  fn estimate_gas(&self, req: CallRequest, block: Option<BlockNumber>) -> Result<U256> {
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

  fn block(&self, block: BlockId, include_txs: bool) -> Result<Block> {
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

  fn storage(&self, address: Address, idx: U256, block: Option<BlockNumber>) -> Result<H256> {
    let address = helpers::serialize(&address);
    let idx = helpers::serialize(&idx);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

    self.transport.execute("eth_getStorageAt", Some(vec![address, idx, block]))
      .and_then(helpers::to_h256)
      .boxed()
  }

  fn transaction_count(&self, address: Address, block: Option<BlockNumber>) -> Result<U256> {
    let address = helpers::serialize(&address);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

   self.transport.execute("eth_getTransactionCount", Some(vec![address, block]))
      .and_then(helpers::to_u256)
      .boxed()
  }

  fn transaction(&self, id: TransactionId) -> Result<Option<Transaction>> {
    let result = match id {
      TransactionId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getTransactionByHash", Some(vec![hash]))
      },
      TransactionId::Block(BlockId::Hash(hash), index) => {
        let hash = helpers::serialize(&hash);
        let idx = helpers::serialize(&index);
        self.transport.execute("eth_getTransactionByBlockHashAndIndex", Some(vec![hash, idx]))
      },
      TransactionId::Block(BlockId::Number(number), index) => {
        let number = helpers::serialize(&number);
        let idx = helpers::serialize(&index);
        self.transport.execute("eth_getTransactionByBlockNumberAndIndex", Some(vec![number, idx]))
      },
    };

    result
      .and_then(|_| Ok(Some(())))
      .boxed()
  }

  fn transaction_receipt(&self, hash: H256) -> Result<Option<TransactionReceipt>> {
    let hash = helpers::serialize(&hash);
  
    self.transport.execute("eth_getTransactionReceipt", Some(vec![hash]))
      .and_then(|_| Ok(Some(())))
      .boxed()
  }

  fn uncle(&self, block: BlockId, index: Index) -> Result<Option<Block>> {
    let index = helpers::serialize(&index);

    let result = match block {
      BlockId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getUncleByBlockHashAndIndex", Some(vec![hash, index]))
      },
      BlockId::Number(num) => {
        let num = helpers::serialize(&num);
        self.transport.execute("eth_getUncleByBlockNumberAndIndex", Some(vec![num, index]))
      },
    };
  
    result
      .and_then(|_| Ok(Some(())))
      .boxed()
  }

  fn uncle_count(&self, block: BlockId) -> Result<Option<U256>> {
    let result = match block {
      BlockId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getUncleCountByBlockHash", Some(vec![hash]))
      },
      BlockId::Number(num) => {
        let num = helpers::serialize(&num);
        self.transport.execute("eth_getUncleCountByBlockNumber", Some(vec![num]))
      },
    };

    result
      .and_then(helpers::to_u256_option)
      .boxed()
  }

  fn work(&self) -> Result<Work> {
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

  fn send_raw_transaction(&self, rlp: Bytes) -> Result<H256> {
    let rlp = helpers::serialize(&rlp);
    self.transport.execute("eth_sendRawTransaction", Some(vec![rlp]))
      .and_then(helpers::to_h256)
      .boxed()
  }

  fn send_transaction(&self, tx: TransactionRequest) -> Result<H256> {
    let tx = helpers::serialize(&tx);
    self.transport.execute("eth_sendTransaction", Some(vec![tx]))
      .and_then(helpers::to_h256)
      .boxed()
  }

  fn sign(&self, address: Address, data: Bytes) -> Result<H512> {
    let address = helpers::serialize(&address);
    let data = helpers::serialize(&data);
    self.transport.execute("eth_sign", Some(vec![address, data]))
      .and_then(helpers::to_h512)
      .boxed()
  }

  fn submit_hashrate(&self, rate: U256, id: H256) -> Result<bool> {
    let rate = helpers::serialize(&rate);
    let id = helpers::serialize(&id);
    self.transport.execute("eth_submitHashrate", Some(vec![rate, id]))
      .and_then(helpers::to_bool)
      .boxed()
  }

  fn submit_work(&self, nonce: H64, pow_hash: H256, mix_hash: H256) -> Result<bool> {
    let nonce = helpers::serialize(&nonce);
    let pow_hash = helpers::serialize(&pow_hash);
    let mix_hash = helpers::serialize(&mix_hash);
    self.transport.execute("eth_submitWork", Some(vec![nonce, pow_hash, mix_hash]))
      .and_then(helpers::to_bool)
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
  use types::{
    BlockId, BlockNumber, Bytes,
    CallRequest,
    TransactionId, TransactionRequest,
  };
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
    Eth:call, CallRequest {
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
    Eth:estimate_gas, CallRequest {
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

  rpc_test! (
    Eth:storage, "0x123", "0x456", None
    =>
    "eth_getStorageAt", vec![r#""0x123""#, r#""0x456""#, r#""latest""#];
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:transaction_count, "0x123", None
    =>
    "eth_getTransactionCount", vec![r#""0x123""#, r#""latest""#];
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:transaction:tx_by_hash, TransactionId::Hash("0x123".into())
    =>
    "eth_getTransactionByHash", vec![r#""0x123""#];
    Value::Null => Some(())
  );

  rpc_test! (
    Eth:transaction:tx_by_block_hash_and_index, TransactionId::Block(
      BlockId::Hash("0x123".into()),
      "0x5".into()
    )
    =>
    "eth_getTransactionByBlockHashAndIndex", vec![r#""0x123""#, r#""0x5""#];
    Value::Null => Some(())
  );

  rpc_test! (
    Eth:transaction:tx_by_block_no_and_index, TransactionId::Block(
      BlockNumber::Pending.into(),
      "0x5".into()
    )
    =>
    "eth_getTransactionByBlockNumberAndIndex", vec![r#""pending""#, r#""0x5""#];
    Value::Null => Some(())
  );

  rpc_test! (
    Eth:transaction_receipt, "0x123".to_owned()
    =>
    "eth_getTransactionReceipt", vec![r#""0x123""#];
    Value::Null => Some(())
  );

  rpc_test! (
    Eth:uncle:uncle_by_hash, BlockId::Hash("0x123".into()), "0x5"
    =>
    "eth_getUncleByBlockHashAndIndex", vec![r#""0x123""#, r#""0x5""#];
    Value::Null => Some(())
  );

  rpc_test! (
    Eth:uncle:uncle_by_no, BlockNumber::Earliest, "0x5"
    =>
    "eth_getUncleByBlockNumberAndIndex", vec![r#""earliest""#, r#""0x5""#];
    Value::Null => Some(())
  );

  rpc_test! (
    Eth:uncle_count:uncle_count_by_hash, BlockId::Hash("0x123".into())
    =>
    "eth_getUncleCountByBlockHash", vec![r#""0x123""#];
    Value::String("0x123".into())=> Some("0x123".into())
  );

  rpc_test! (
    Eth:uncle_count:uncle_count_by_no, BlockNumber::Earliest
    =>
    "eth_getUncleCountByBlockNumber", vec![r#""earliest""#];
    Value::Null => None
  );

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
  rpc_test! (
    Eth:new_pending_transaction_filter => "eth_newPendingTransactionFilter";
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:protocol_version => "eth_protocolVersion";
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:send_raw_transaction, Bytes(vec![1, 2, 3, 4])
    =>
    "eth_sendRawTransaction", vec![r#""0x01020304""#];
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:send_transaction, TransactionRequest {
      from: "0x123".into(), to: Some("0x123".into()),
      gas: None, gas_price: Some("0x1".into()),
      value: Some("0x1".into()), data: None,
      nonce: None, min_block: None,
    }
    =>
    "eth_sendTransaction", vec![r#"{"from":"0x123","to":"0x123","gasPrice":"0x1","value":"0x1"}"#];
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:sign, "0x123", Bytes(vec![1, 2, 3, 4])
    =>
    "eth_sign", vec![r#""0x123""#, r#""0x01020304""#];
    Value::String("0x123".into()) => "0x123"
  );

  rpc_test! (
    Eth:submit_hashrate, "0x123", "0x456"
    =>
    "eth_submitHashrate", vec![r#""0x123""#, r#""0x456""#];
    Value::Bool(true) => true
  );

  rpc_test! (
    Eth:submit_work, "0x123", "0x456", "0x789"
    =>
    "eth_submitWork", vec![r#""0x123""#, r#""0x456""#, r#""0x789""#];
    Value::Bool(true) => true
  );
  
  rpc_test! (
    Eth:syncing => "eth_syncing";
    Value::Bool(true) => true
  );
}
