//! `Eth` namespace

use api::Namespace;
use helpers::{self, CallResult};
use types::{
  Address, Block, BlockId, BlockNumber, Bytes, CallRequest,
  H64, H256, H512, Index,
  Transaction, TransactionId, TransactionReceipt, TransactionRequest,
  U256, Work,
};
use {Transport};

/// `Eth` namespace
pub struct Eth<'a, T: 'a> {
  transport: &'a T,
}

impl<'a, T: Transport + 'a> Namespace<'a, T> for Eth<'a, T> {
  fn new(transport: &'a T) -> Self where Self: Sized {
    Eth {
      transport: transport,
    }
  }
}

impl<'a, T: Transport + 'a> Eth<'a, T> {
  /// Get list of available accounts.
  pub fn accounts(&self) -> CallResult<Vec<Address>, T::Out> {
    CallResult::new(self.transport.execute("eth_accounts", vec![]))
  }

  /// Get current block number
  pub fn block_number(&self) -> CallResult<U256, T::Out> {
    CallResult::new(self.transport.execute("eth_blockNumber", vec![]))
  }

  /// Call a constant method of contract without changing the state of the blockchain.
  pub fn call(&self, req: CallRequest, block: Option<BlockNumber>) -> CallResult<Bytes, T::Out> {
    let req = helpers::serialize(&req);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

    CallResult::new(self.transport.execute("eth_call", vec![req, block]))
  }

  /// Get coinbase address
  pub fn coinbase(&self) -> CallResult<Address, T::Out> {
    CallResult::new(self.transport.execute("eth_coinbase", vec![]))
  }

  /// Compile LLL
  pub fn compile_lll(&self, code: String) -> CallResult<Bytes, T::Out> {
    let code = helpers::serialize(&code);
    CallResult::new(self.transport.execute("eth_compileLLL", vec![code]))
  }

  /// Compile Solidity
  pub fn compile_solidity(&self, code: String) -> CallResult<Bytes, T::Out> {
    let code = helpers::serialize(&code);
    CallResult::new(self.transport.execute("eth_compileSolidity", vec![code]))
  }

  /// Compile Serpent
  pub fn compile_serpent(&self, code: String) -> CallResult<Bytes, T::Out> {
    let code = helpers::serialize(&code);
    CallResult::new(self.transport.execute("eth_compileSerpent", vec![code]))
  }

  /// Call a contract without changing the state of the blockchain to estimate gas usage.
  pub fn estimate_gas(&self, req: CallRequest, block: Option<BlockNumber>) -> CallResult<U256, T::Out> {
    let req = helpers::serialize(&req);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

    CallResult::new(self.transport.execute("eth_estimateGas", vec![req, block]))
  }

  /// Get current recommended gas price
  pub fn gas_price(&self) -> CallResult<U256, T::Out> {
    CallResult::new(self.transport.execute("eth_gasPrice", vec![]))
  }

  /// Get balance of given address
  pub fn balance(&self, address: Address, block: Option<BlockNumber>) -> CallResult<U256, T::Out> {
    let address = helpers::serialize(&address);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

    CallResult::new(self.transport.execute("eth_getBalance", vec![address, block]))
  }

  /// Get block details
  pub fn block(&self, block: BlockId, include_txs: bool) -> CallResult<Block, T::Out> {
    let include_txs = helpers::serialize(&include_txs);

    let result = match block {
      BlockId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getBlockByHash", vec![hash, include_txs])
      },
      BlockId::Number(num) => {
        let num = helpers::serialize(&num);
        self.transport.execute("eth_getBlockByNumber", vec![num, include_txs])
      },
    };

    CallResult::new(result)
  }

  /// Get number of transactions in block
  pub fn block_transaction_count(&self, block: BlockId) -> CallResult<Option<U256>, T::Out> {
    let result = match block {
      BlockId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getBlockTransactionCountByHash", vec![hash])
      },
      BlockId::Number(num) => {
        let num = helpers::serialize(&num);
        self.transport.execute("eth_getBlockTransactionCountByNumber", vec![num])
      },
    };

    CallResult::new(result)
  }

  /// Get code under given address
  pub fn code(&self, address: Address, block: Option<BlockNumber>) -> CallResult<Bytes, T::Out> {
    let address = helpers::serialize(&address);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

    CallResult::new(self.transport.execute("eth_getCode", vec![address, block]))
  }

  /// Get supported compilers
  pub fn compilers(&self) -> CallResult<Vec<String>, T::Out> {
    CallResult::new(self.transport.execute("eth_getCompilers", vec![]))
  }

  /// Get storage entry
  pub fn storage(&self, address: Address, idx: U256, block: Option<BlockNumber>) -> CallResult<H256, T::Out> {
    let address = helpers::serialize(&address);
    let idx = helpers::serialize(&idx);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

    CallResult::new(self.transport.execute("eth_getStorageAt", vec![address, idx, block]))
  }

  /// Get nonce
  pub fn transaction_count(&self, address: Address, block: Option<BlockNumber>) -> CallResult<U256, T::Out> {
    let address = helpers::serialize(&address);
    let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

   CallResult::new(self.transport.execute("eth_getTransactionCount", vec![address, block]))
  }

  /// Get transaction
  pub fn transaction(&self, id: TransactionId) -> CallResult<Option<Transaction>, T::Out> {
    let result = match id {
      TransactionId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getTransactionByHash", vec![hash])
      },
      TransactionId::Block(BlockId::Hash(hash), index) => {
        let hash = helpers::serialize(&hash);
        let idx = helpers::serialize(&index);
        self.transport.execute("eth_getTransactionByBlockHashAndIndex", vec![hash, idx])
      },
      TransactionId::Block(BlockId::Number(number), index) => {
        let number = helpers::serialize(&number);
        let idx = helpers::serialize(&index);
        self.transport.execute("eth_getTransactionByBlockNumberAndIndex", vec![number, idx])
      },
    };

    CallResult::new(result)
  }

  /// Get transaction receipt
  pub fn transaction_receipt(&self, hash: H256) -> CallResult<Option<TransactionReceipt>, T::Out> {
    let hash = helpers::serialize(&hash);

    CallResult::new(self.transport.execute("eth_getTransactionReceipt", vec![hash]))
  }

  /// Get uncle
  pub fn uncle(&self, block: BlockId, index: Index) -> CallResult<Option<Block>, T::Out> {
    let index = helpers::serialize(&index);

    let result = match block {
      BlockId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getUncleByBlockHashAndIndex", vec![hash, index])
      },
      BlockId::Number(num) => {
        let num = helpers::serialize(&num);
        self.transport.execute("eth_getUncleByBlockNumberAndIndex", vec![num, index])
      },
    };

    CallResult::new(result)
  }

  /// Get uncle count in block
  pub fn uncle_count(&self, block: BlockId) -> CallResult<Option<U256>, T::Out> {
    let result = match block {
      BlockId::Hash(hash) => {
        let hash = helpers::serialize(&hash);
        self.transport.execute("eth_getUncleCountByBlockHash", vec![hash])
      },
      BlockId::Number(num) => {
        let num = helpers::serialize(&num);
        self.transport.execute("eth_getUncleCountByBlockNumber", vec![num])
      },
    };

    CallResult::new(result)
  }

  /// Get work package
  pub fn work(&self) -> CallResult<Work, T::Out> {
    CallResult::new(self.transport.execute("eth_getWork", vec![]))
  }

  /// Get hash rate
  pub fn hashrate(&self) -> CallResult<U256, T::Out> {
    CallResult::new(self.transport.execute("eth_hashrate", vec![]))
  }

  /// Get mining status
  pub fn mining(&self) -> CallResult<bool, T::Out> {
    CallResult::new(self.transport.execute("eth_mining", vec![]))
  }

  /// Start new block filter
  pub fn new_block_filter(&self) -> CallResult<U256, T::Out> {
    CallResult::new(self.transport.execute("eth_newBlockFilter", vec![]))
  }

  /// Start new pending transaction filter
  pub fn new_pending_transaction_filter(&self) -> CallResult<U256, T::Out> {
    CallResult::new(self.transport.execute("eth_newPendingTransactionFilter", vec![]))
  }

  /// Start new pending transaction filter
  pub fn protocol_version(&self) -> CallResult<String, T::Out> {
    CallResult::new(self.transport.execute("eth_protocolVersion", vec![]))
  }

  /// Sends a rlp-encoded signed transaction
  pub fn send_raw_transaction(&self, rlp: Bytes) -> CallResult<H256, T::Out> {
    let rlp = helpers::serialize(&rlp);
    CallResult::new(self.transport.execute("eth_sendRawTransaction", vec![rlp]))
  }

  /// Sends a transaction transaction
  pub fn send_transaction(&self, tx: TransactionRequest) -> CallResult<H256, T::Out> {
    let tx = helpers::serialize(&tx);
    CallResult::new(self.transport.execute("eth_sendTransaction", vec![tx]))
  }

  /// Signs a hash of given data
  pub fn sign(&self, address: Address, data: Bytes) -> CallResult<H512, T::Out> {
    let address = helpers::serialize(&address);
    let data = helpers::serialize(&data);
    CallResult::new(self.transport.execute("eth_sign", vec![address, data]))
  }

  /// Submit hashrate of external miner
  pub fn submit_hashrate(&self, rate: U256, id: H256) -> CallResult<bool, T::Out> {
    let rate = helpers::serialize(&rate);
    let id = helpers::serialize(&id);
    CallResult::new(self.transport.execute("eth_submitHashrate", vec![rate, id]))
  }

  /// Submit work of external miner
  pub fn submit_work(&self, nonce: H64, pow_hash: H256, mix_hash: H256) -> CallResult<bool, T::Out> {
    let nonce = helpers::serialize(&nonce);
    let pow_hash = helpers::serialize(&pow_hash);
    let mix_hash = helpers::serialize(&mix_hash);
    CallResult::new(self.transport.execute("eth_submitWork", vec![nonce, pow_hash, mix_hash]))
  }

  // TODO [ToDr] Proper type?
  /// Get syncing status
  pub fn syncing(&self) -> CallResult<bool, T::Out> {
    CallResult::new(self.transport.execute("eth_syncing", vec![]))
  }
}

#[cfg(test)]
mod tests {
  use futures::Future;

  use api::Namespace;
  use types::{
    BlockId, BlockNumber, Bytes,
    CallRequest,
    TransactionId, TransactionRequest,
  };
  use rpc::Value;

  use super::Eth;

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
    Value::Array(vec![]) => Some(())
  );

  rpc_test! (
    Eth:transaction:tx_by_block_hash_and_index, TransactionId::Block(
      BlockId::Hash("0x123".into()),
      "0x5".into()
    )
    =>
    "eth_getTransactionByBlockHashAndIndex", vec![r#""0x123""#, r#""0x5""#];
    Value::Array(vec![]) => Some(())
  );

  rpc_test! (
    Eth:transaction:tx_by_block_no_and_index, TransactionId::Block(
      BlockNumber::Pending.into(),
      "0x5".into()
    )
    =>
    "eth_getTransactionByBlockNumberAndIndex", vec![r#""pending""#, r#""0x5""#];
    Value::Array(vec![]) => Some(())
  );

  rpc_test! (
    Eth:transaction_receipt, "0x123".to_owned()
    =>
    "eth_getTransactionReceipt", vec![r#""0x123""#];
    Value::Array(vec![]) => Some(())
  );

  rpc_test! (
    Eth:uncle:uncle_by_hash, BlockId::Hash("0x123".into()), "0x5"
    =>
    "eth_getUncleByBlockHashAndIndex", vec![r#""0x123""#, r#""0x5""#];
    Value::Array(vec![]) => Some(())
  );

  rpc_test! (
    Eth:uncle:uncle_by_no, BlockNumber::Earliest, "0x5"
    =>
    "eth_getUncleByBlockNumberAndIndex", vec![r#""earliest""#, r#""0x5""#];
    Value::Array(vec![]) => Some(())
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
