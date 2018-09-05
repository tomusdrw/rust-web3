//! `Eth` namespace

use api::Namespace;
use helpers::{self, CallFuture};
use types::{Address, Block, BlockId, BlockNumber, Bytes, CallRequest, H256, H520, H64, Index, SyncState, Transaction, TransactionId, TransactionReceipt, TransactionRequest, U256, Work, Filter, Log};
use Transport;

/// `Eth` namespace
#[derive(Debug, Clone)]
pub struct Eth<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for Eth<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        Eth { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Eth<T> {
    /// Get list of available accounts.
    pub fn accounts(&self) -> CallFuture<Vec<Address>, T::Out> {
        CallFuture::new(self.transport.execute("eth_accounts", vec![]))
    }

    /// Get current block number
    pub fn block_number(&self) -> CallFuture<U256, T::Out> {
        CallFuture::new(self.transport.execute("eth_blockNumber", vec![]))
    }

    /// Call a constant method of contract without changing the state of the blockchain.
    pub fn call(&self, req: CallRequest, block: Option<BlockNumber>) -> CallFuture<Bytes, T::Out> {
        let req = helpers::serialize(&req);
        let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

        CallFuture::new(self.transport.execute("eth_call", vec![req, block]))
    }

    /// Get coinbase address
    pub fn coinbase(&self) -> CallFuture<Address, T::Out> {
        CallFuture::new(self.transport.execute("eth_coinbase", vec![]))
    }

    /// Compile LLL
    pub fn compile_lll(&self, code: String) -> CallFuture<Bytes, T::Out> {
        let code = helpers::serialize(&code);
        CallFuture::new(self.transport.execute("eth_compileLLL", vec![code]))
    }

    /// Compile Solidity
    pub fn compile_solidity(&self, code: String) -> CallFuture<Bytes, T::Out> {
        let code = helpers::serialize(&code);
        CallFuture::new(self.transport.execute("eth_compileSolidity", vec![code]))
    }

    /// Compile Serpent
    pub fn compile_serpent(&self, code: String) -> CallFuture<Bytes, T::Out> {
        let code = helpers::serialize(&code);
        CallFuture::new(self.transport.execute("eth_compileSerpent", vec![code]))
    }

    /// Call a contract without changing the state of the blockchain to estimate gas usage.
    pub fn estimate_gas(&self, req: CallRequest, block: Option<BlockNumber>) -> CallFuture<U256, T::Out> {
        let req = helpers::serialize(&req);
        let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

        CallFuture::new(self.transport.execute("eth_estimateGas", vec![req, block]))
    }

    /// Get current recommended gas price
    pub fn gas_price(&self) -> CallFuture<U256, T::Out> {
        CallFuture::new(self.transport.execute("eth_gasPrice", vec![]))
    }

    /// Get balance of given address
    pub fn balance(&self, address: Address, block: Option<BlockNumber>) -> CallFuture<U256, T::Out> {
        let address = helpers::serialize(&address);
        let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

        CallFuture::new(
            self.transport
                .execute("eth_getBalance", vec![address, block]),
        )
    }

    /// Get all logs matching a given filter object
    pub fn logs(&self, filter: Filter) -> CallFuture<Vec<Log>,T::Out> {
        let filter = helpers::serialize(&filter);
        CallFuture::new(self.transport.execute("eth_getLogs",vec![filter]))
    }

    /// Get block details with transaction hashes.
    pub fn block(&self, block: BlockId) -> CallFuture<Option<Block<H256>>, T::Out> {
        let include_txs = helpers::serialize(&false);

        let result = match block {
            BlockId::Hash(hash) => {
                let hash = helpers::serialize(&hash);
                self.transport
                    .execute("eth_getBlockByHash", vec![hash, include_txs])
            }
            BlockId::Number(num) => {
                let num = helpers::serialize(&num);
                self.transport
                    .execute("eth_getBlockByNumber", vec![num, include_txs])
            }
        };

        CallFuture::new(result)
    }

    /// Get block details with full transaction objects.
    pub fn block_with_txs(&self, block: BlockId) -> CallFuture<Option<Block<Transaction>>, T::Out> {
        let include_txs = helpers::serialize(&true);

        let result = match block {
            BlockId::Hash(hash) => {
                let hash = helpers::serialize(&hash);
                self.transport
                    .execute("eth_getBlockByHash", vec![hash, include_txs])
            }
            BlockId::Number(num) => {
                let num = helpers::serialize(&num);
                self.transport
                    .execute("eth_getBlockByNumber", vec![num, include_txs])
            }
        };

        CallFuture::new(result)
    }

    /// Get number of transactions in block
    pub fn block_transaction_count(&self, block: BlockId) -> CallFuture<Option<U256>, T::Out> {
        let result = match block {
            BlockId::Hash(hash) => {
                let hash = helpers::serialize(&hash);
                self.transport
                    .execute("eth_getBlockTransactionCountByHash", vec![hash])
            }
            BlockId::Number(num) => {
                let num = helpers::serialize(&num);
                self.transport
                    .execute("eth_getBlockTransactionCountByNumber", vec![num])
            }
        };

        CallFuture::new(result)
    }

    /// Get code under given address
    pub fn code(&self, address: Address, block: Option<BlockNumber>) -> CallFuture<Bytes, T::Out> {
        let address = helpers::serialize(&address);
        let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

        CallFuture::new(self.transport.execute("eth_getCode", vec![address, block]))
    }

    /// Get supported compilers
    pub fn compilers(&self) -> CallFuture<Vec<String>, T::Out> {
        CallFuture::new(self.transport.execute("eth_getCompilers", vec![]))
    }

    /// Get storage entry
    pub fn storage(&self, address: Address, idx: U256, block: Option<BlockNumber>) -> CallFuture<H256, T::Out> {
        let address = helpers::serialize(&address);
        let idx = helpers::serialize(&idx);
        let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

        CallFuture::new(
            self.transport
                .execute("eth_getStorageAt", vec![address, idx, block]),
        )
    }

    /// Get nonce
    pub fn transaction_count(&self, address: Address, block: Option<BlockNumber>) -> CallFuture<U256, T::Out> {
        let address = helpers::serialize(&address);
        let block = helpers::serialize(&block.unwrap_or(BlockNumber::Latest));

        CallFuture::new(
            self.transport
                .execute("eth_getTransactionCount", vec![address, block]),
        )
    }

    /// Get transaction
    pub fn transaction(&self, id: TransactionId) -> CallFuture<Option<Transaction>, T::Out> {
        let result = match id {
            TransactionId::Hash(hash) => {
                let hash = helpers::serialize(&hash);
                self.transport
                    .execute("eth_getTransactionByHash", vec![hash])
            }
            TransactionId::Block(BlockId::Hash(hash), index) => {
                let hash = helpers::serialize(&hash);
                let idx = helpers::serialize(&index);
                self.transport
                    .execute("eth_getTransactionByBlockHashAndIndex", vec![hash, idx])
            }
            TransactionId::Block(BlockId::Number(number), index) => {
                let number = helpers::serialize(&number);
                let idx = helpers::serialize(&index);
                self.transport
                    .execute("eth_getTransactionByBlockNumberAndIndex", vec![number, idx])
            }
        };

        CallFuture::new(result)
    }

    /// Get transaction receipt
    pub fn transaction_receipt(&self, hash: H256) -> CallFuture<Option<TransactionReceipt>, T::Out> {
        let hash = helpers::serialize(&hash);

        CallFuture::new(
            self.transport
                .execute("eth_getTransactionReceipt", vec![hash]),
        )
    }

    /// Get uncle by block ID and uncle index -- transactions only has hashes.
    pub fn uncle(&self, block: BlockId, index: Index) -> CallFuture<Option<Block<H256>>, T::Out> {
        let index = helpers::serialize(&index);

        let result = match block {
            BlockId::Hash(hash) => {
                let hash = helpers::serialize(&hash);
                self.transport
                    .execute("eth_getUncleByBlockHashAndIndex", vec![hash, index])
            }
            BlockId::Number(num) => {
                let num = helpers::serialize(&num);
                self.transport
                    .execute("eth_getUncleByBlockNumberAndIndex", vec![num, index])
            }
        };

        CallFuture::new(result)
    }

    /// Get uncle count in block
    pub fn uncle_count(&self, block: BlockId) -> CallFuture<Option<U256>, T::Out> {
        let result = match block {
            BlockId::Hash(hash) => {
                let hash = helpers::serialize(&hash);
                self.transport
                    .execute("eth_getUncleCountByBlockHash", vec![hash])
            }
            BlockId::Number(num) => {
                let num = helpers::serialize(&num);
                self.transport
                    .execute("eth_getUncleCountByBlockNumber", vec![num])
            }
        };

        CallFuture::new(result)
    }

    /// Get work package
    pub fn work(&self) -> CallFuture<Work, T::Out> {
        CallFuture::new(self.transport.execute("eth_getWork", vec![]))
    }

    /// Get hash rate
    pub fn hashrate(&self) -> CallFuture<U256, T::Out> {
        CallFuture::new(self.transport.execute("eth_hashrate", vec![]))
    }

    /// Get mining status
    pub fn mining(&self) -> CallFuture<bool, T::Out> {
        CallFuture::new(self.transport.execute("eth_mining", vec![]))
    }

    /// Start new block filter
    pub fn new_block_filter(&self) -> CallFuture<U256, T::Out> {
        CallFuture::new(self.transport.execute("eth_newBlockFilter", vec![]))
    }

    /// Start new pending transaction filter
    pub fn new_pending_transaction_filter(&self) -> CallFuture<U256, T::Out> {
        CallFuture::new(
            self.transport
                .execute("eth_newPendingTransactionFilter", vec![]),
        )
    }

    /// Start new pending transaction filter
    pub fn protocol_version(&self) -> CallFuture<String, T::Out> {
        CallFuture::new(self.transport.execute("eth_protocolVersion", vec![]))
    }

    /// Sends a rlp-encoded signed transaction
    pub fn send_raw_transaction(&self, rlp: Bytes) -> CallFuture<H256, T::Out> {
        let rlp = helpers::serialize(&rlp);
        CallFuture::new(self.transport.execute("eth_sendRawTransaction", vec![rlp]))
    }

    /// Sends a transaction transaction
    pub fn send_transaction(&self, tx: TransactionRequest) -> CallFuture<H256, T::Out> {
        let tx = helpers::serialize(&tx);
        CallFuture::new(self.transport.execute("eth_sendTransaction", vec![tx]))
    }

    /// Signs a hash of given data
    pub fn sign(&self, address: Address, data: Bytes) -> CallFuture<H520, T::Out> {
        let address = helpers::serialize(&address);
        let data = helpers::serialize(&data);
        CallFuture::new(self.transport.execute("eth_sign", vec![address, data]))
    }

    /// Submit hashrate of external miner
    pub fn submit_hashrate(&self, rate: U256, id: H256) -> CallFuture<bool, T::Out> {
        let rate = helpers::serialize(&rate);
        let id = helpers::serialize(&id);
        CallFuture::new(self.transport.execute("eth_submitHashrate", vec![rate, id]))
    }

    /// Submit work of external miner
    pub fn submit_work(&self, nonce: H64, pow_hash: H256, mix_hash: H256) -> CallFuture<bool, T::Out> {
        let nonce = helpers::serialize(&nonce);
        let pow_hash = helpers::serialize(&pow_hash);
        let mix_hash = helpers::serialize(&mix_hash);
        CallFuture::new(
            self.transport
                .execute("eth_submitWork", vec![nonce, pow_hash, mix_hash]),
        )
    }

    /// Get syncing status
    pub fn syncing(&self) -> CallFuture<SyncState, T::Out> {
        CallFuture::new(self.transport.execute("eth_syncing", vec![]))
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;

    use api::Namespace;
    use types::{Block, BlockId, BlockNumber, Bytes, CallRequest, H256, SyncInfo, SyncState, Transaction, TransactionId, TransactionReceipt, TransactionRequest, Work, FilterBuilder, Log};
    use rpc::Value;

    use super::Eth;

    // taken from RPC docs.
    const EXAMPLE_BLOCK: &'static str = r#"{
    "number": "0x1b4",
    "hash": "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331",
    "parentHash": "0x9646252be9520f6e71339a8df9c55e4d7619deeb018d2a3f2d21fc165dde5eb5",
    "sealFields": [
      "0xe04d296d2460cfb8472af2c5fd05b5a214109c25688d3704aed5484f9a7792f2",
      "0x0000000000000042"
    ],
    "sha3Uncles": "0x1dcc4de8dec75d7aab85b567b6ccd41ad312451b948a7413f0a142fd40d49347",
    "logsBloom":  "0x0e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d15273310e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d15273310e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d15273310e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d15273310e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d15273310e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d15273310e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d15273310e670ec64341771606e55d6b4ca35a1a6b75ee3d5145a99d05921026d1527331",
    "transactionsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    "receiptsRoot": "0x56e81f171bcc55a6ff8345e692c0f86e5b48e01b996cadc001622fb5e363b421",
    "stateRoot": "0xd5855eb08b3387c0af375e9cdb6acfc05eb8f519e419b874b6ff2ffda7ed1dff",
    "miner": "0x4e65fda2159562a496f9f3522f89122a3088497a",
    "difficulty": "0x27f07",
    "totalDifficulty": "0x27f07",
    "extraData": "0x0000000000000000000000000000000000000000000000000000000000000000",
    "size": "0x27f07",
    "gasLimit": "0x9f759",
    "minGasPrice": "0x9f759",
    "gasUsed": "0x9f759",
    "timestamp": "0x54e34e8e",
    "transactions": [],
    "uncles": []
  }"#;

    // taken from RPC docs, but with leading `00` added to `blockHash`
    // and `transactionHash` fields because RPC docs currently show 
    // 31-byte values in both positions (must be 32 bytes).
    const EXAMPLE_LOG: &'static str = r#"{
    "logIndex": "0x1",
    "blockNumber":"0x1b4",
    "blockHash": "0x008216c5785ac562ff41e2dcfdf5785ac562ff41e2dcfdf829c5a142f1fccd7d",
    "transactionHash":  "0x00df829c5a142f1fccd7d8216c5785ac562ff41e2dcfdf5785ac562ff41e2dcf",
    "transactionIndex": "0x0",
    "address": "0x16c5785ac562ff41e2dcfdf829c5a142f1fccd7d",
    "data":"0x0000000000000000000000000000000000000000000000000000000000000000",
    "topics": ["0x59ebeb90bc63057b6515673c3ecf9438e5058bca0f92585014eced636878c9a5"]
  }"#;

    // taken from RPC docs.
    const EXAMPLE_TX: &'static str = r#"{
    "hash": "0xc6ef2fc5426d6ad6fd9e2a26abeab0aa2411b7ab17f30a99d3cb96aed1d1055b",
    "nonce": "0x0",
    "blockHash": "0xbeab0aa2411b7ab17f30a99d3cb9c6ef2fc5426d6ad6fd9e2a26a6aed1d1055b",
    "blockNumber": "0x15df",
    "transactionIndex": "0x1",
    "from": "0x407d73d8a49eeb85d32cf465507dd71d507100c1",
    "to":   "0x85dd43d8a49eeb85d32cf465507dd71d507100c1",
    "value": "0x7f110",
    "gas": "0x7f110",
    "gasPrice": "0x09184e72a000",
    "input": "0x603880600c6000396000f300603880600c6000396000f3603880600c6000396000f360"
  }"#;

    // taken from RPC docs.
    const EXAMPLE_RECEIPT: &'static str = r#"{
    "hash": "0xb903239f8543d04b5dc1ba6579132b143087c68db1b2168786408fcbce568238",
    "index": "0x1",
    "transactionHash": "0xb903239f8543d04b5dc1ba6579132b143087c68db1b2168786408fcbce568238",
    "transactionIndex": "0x1",
    "blockNumber": "0xb",
    "blockHash": "0xc6ef2fc5426d6ad6fd9e2a26abeab0aa2411b7ab17f30a99d3cb96aed1d1055b",
    "cumulativeGasUsed": "0x33bc",
    "gasUsed": "0x4dc",
    "contractAddress": "0xb60e8dd61c5d32be8058bb8eb970870f07233155",
    "logs": []
  }"#;

    rpc_test! (
    Eth:accounts => "eth_accounts";
    Value::Array(vec![Value::String("0x0000000000000000000000000000000000000123".into())]) => vec![0x123.into()]
  );

    rpc_test! (
    Eth:block_number => "eth_blockNumber";
    Value::String("0x123".into()) => 0x123
  );

    rpc_test! (
    Eth:call, CallRequest {
      from: None, to: 0x123.into(),
      gas: None, gas_price: None,
      value: Some(0x1.into()), data: None,
    }, None
    =>
    "eth_call", vec![r#"{"to":"0x0000000000000000000000000000000000000123","value":"0x1"}"#, r#""latest""#];
    Value::String("0x010203".into()) => Bytes(vec![1, 2, 3])
  );

    rpc_test! (
    Eth:coinbase => "eth_coinbase";
    Value::String("0x0000000000000000000000000000000000000123".into()) => 0x123
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
      from: None, to: 0x123.into(),
      gas: None, gas_price: None,
      value: Some(0x1.into()), data: None,
    }, None
    =>
    "eth_estimateGas", vec![r#"{"to":"0x0000000000000000000000000000000000000123","value":"0x1"}"#, r#""latest""#];
    Value::String("0x123".into()) => 0x123
  );

    rpc_test! (
    Eth:gas_price => "eth_gasPrice";
    Value::String("0x123".into()) => 0x123
  );

    rpc_test! (
    Eth:balance, 0x123, None
    =>
    "eth_getBalance", vec![r#""0x0000000000000000000000000000000000000123""#, r#""latest""#];
    Value::String("0x123".into()) => 0x123
  );

    rpc_test! (
    Eth:logs, FilterBuilder::default().build() => "eth_getLogs", vec!["{}"];
    Value::Array(vec![::serde_json::from_str(EXAMPLE_LOG).unwrap()])
    => vec![::serde_json::from_str::<Log>(EXAMPLE_LOG).unwrap()]
  );

    rpc_test! (
    Eth:block:block_by_hash, BlockId::Hash(0x123.into())
    =>
    "eth_getBlockByHash", vec![r#""0x0000000000000000000000000000000000000000000000000000000000000123""#, r#"false"#];
    ::serde_json::from_str(EXAMPLE_BLOCK).unwrap()
    => Some(::serde_json::from_str::<Block<H256>>(EXAMPLE_BLOCK).unwrap())
  );

    rpc_test! (
    Eth:block, BlockNumber::Pending
    =>
    "eth_getBlockByNumber", vec![r#""pending""#, r#"false"#];
    ::serde_json::from_str(EXAMPLE_BLOCK).unwrap()
    => Some(::serde_json::from_str::<Block<H256>>(EXAMPLE_BLOCK).unwrap())
  );

    rpc_test! (
    Eth:block_with_txs, BlockNumber::Pending
    =>
    "eth_getBlockByNumber", vec![r#""pending""#, r#"true"#];
    ::serde_json::from_str(EXAMPLE_BLOCK).unwrap()
    => Some(::serde_json::from_str::<Block<Transaction>>(EXAMPLE_BLOCK).unwrap())
  );

    rpc_test! (
    Eth:block_transaction_count:block_tx_count_by_hash, BlockId::Hash(0x123.into())
    =>
    "eth_getBlockTransactionCountByHash", vec![r#""0x0000000000000000000000000000000000000000000000000000000000000123""#];
    Value::String("0x123".into()) => Some(0x123.into())
  );

    rpc_test! (
    Eth:block_transaction_count, BlockNumber::Pending
    =>
    "eth_getBlockTransactionCountByNumber", vec![r#""pending""#];
    Value::Null => None
  );

    rpc_test! (
    Eth:code, 0x123, Some(BlockNumber::Pending)
    =>
    "eth_getCode", vec![r#""0x0000000000000000000000000000000000000123""#, r#""pending""#];
    Value::String("0x0123".into()) => Bytes(vec![0x1, 0x23])
  );

    rpc_test! (
    Eth:compilers => "eth_getCompilers";
    Value::Array(vec![]) => vec![]
  );

    rpc_test! (
    Eth:storage, 0x123, 0x456, None
    =>
    "eth_getStorageAt", vec![
      r#""0x0000000000000000000000000000000000000123""#,
      r#""0x456""#,
      r#""latest""#
    ];
    Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()) => 0x123
  );

    rpc_test! (
    Eth:transaction_count, 0x123, None
    =>
    "eth_getTransactionCount", vec![r#""0x0000000000000000000000000000000000000123""#, r#""latest""#];
    Value::String("0x123".into()) => 0x123
  );

    rpc_test! (
    Eth:transaction:tx_by_hash, TransactionId::Hash(0x123.into())
    =>
    "eth_getTransactionByHash", vec![r#""0x0000000000000000000000000000000000000000000000000000000000000123""#];
    ::serde_json::from_str(EXAMPLE_TX).unwrap()
    => Some(::serde_json::from_str::<Transaction>(EXAMPLE_TX).unwrap())
  );

    rpc_test! (
    Eth:transaction:tx_by_block_hash_and_index, TransactionId::Block(
      BlockId::Hash(0x123.into()),
      5.into()
    )
    =>
    "eth_getTransactionByBlockHashAndIndex", vec![r#""0x0000000000000000000000000000000000000000000000000000000000000123""#, r#""0x5""#];
    Value::Null => None
  );

    rpc_test! (
    Eth:transaction:tx_by_block_no_and_index, TransactionId::Block(
      BlockNumber::Pending.into(),
      5.into()
    )
    =>
    "eth_getTransactionByBlockNumberAndIndex", vec![r#""pending""#, r#""0x5""#];
    ::serde_json::from_str(EXAMPLE_TX).unwrap()
    => Some(::serde_json::from_str::<Transaction>(EXAMPLE_TX).unwrap())
  );

    rpc_test! (
    Eth:transaction_receipt, 0x123
    =>
    "eth_getTransactionReceipt", vec![r#""0x0000000000000000000000000000000000000000000000000000000000000123""#];
    ::serde_json::from_str(EXAMPLE_RECEIPT).unwrap()
    => Some(::serde_json::from_str::<TransactionReceipt>(EXAMPLE_RECEIPT).unwrap())
  );

    rpc_test! (
    Eth:uncle:uncle_by_hash, BlockId::Hash(0x123.into()), 5
    =>
    "eth_getUncleByBlockHashAndIndex", vec![r#""0x0000000000000000000000000000000000000000000000000000000000000123""#, r#""0x5""#];
    ::serde_json::from_str(EXAMPLE_BLOCK).unwrap()
    => Some(::serde_json::from_str::<Block<H256>>(EXAMPLE_BLOCK).unwrap())
  );

    rpc_test! (
    Eth:uncle:uncle_by_no, BlockNumber::Earliest, 5
    =>
    "eth_getUncleByBlockNumberAndIndex", vec![r#""earliest""#, r#""0x5""#];
    Value::Null => None
  );

    rpc_test! (
    Eth:uncle_count:uncle_count_by_hash, BlockId::Hash(0x123.into())
    =>
    "eth_getUncleCountByBlockHash", vec![r#""0x0000000000000000000000000000000000000000000000000000000000000123""#];
    Value::String("0x123".into())=> Some(0x123.into())
  );

    rpc_test! (
    Eth:uncle_count:uncle_count_by_no, BlockNumber::Earliest
    =>
    "eth_getUncleCountByBlockNumber", vec![r#""earliest""#];
    Value::Null => None
  );

    rpc_test! (
    Eth:work:work_3 => "eth_getWork";
    Value::Array(vec![
      Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()),
      Value::String("0x0000000000000000000000000000000000000000000000000000000000000456".into()),
      Value::String("0x0000000000000000000000000000000000000000000000000000000000000789".into()),
    ]) => Work {
      pow_hash: 0x123.into(),
      seed_hash: 0x456.into(),
      target: 0x789.into(),
      number: None,
    }
  );

    rpc_test! (
    Eth:work:work_4 => "eth_getWork";
    Value::Array(vec![
      Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()),
      Value::String("0x0000000000000000000000000000000000000000000000000000000000000456".into()),
      Value::String("0x0000000000000000000000000000000000000000000000000000000000000789".into()),
      Value::Number(5.into()),
    ]) => Work {
      pow_hash: 0x123.into(),
      seed_hash: 0x456.into(),
      target: 0x789.into(),
      number: Some(5),
    }
  );

    rpc_test! (
    Eth:hashrate => "eth_hashrate";
    Value::String("0x123".into()) => 0x123
  );

    rpc_test! (
    Eth:mining => "eth_mining";
    Value::Bool(true) => true
  );

    rpc_test! (
    Eth:new_block_filter => "eth_newBlockFilter";
    Value::String("0x123".into()) => 0x123
  );
    rpc_test! (
    Eth:new_pending_transaction_filter => "eth_newPendingTransactionFilter";
    Value::String("0x123".into()) => 0x123
  );

    rpc_test! (
    Eth:protocol_version => "eth_protocolVersion";
    Value::String("0x123".into()) => "0x123"
  );

    rpc_test! (
    Eth:send_raw_transaction, Bytes(vec![1, 2, 3, 4])
    =>
    "eth_sendRawTransaction", vec![r#""0x01020304""#];
    Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()) => 0x123
  );

    rpc_test! (
    Eth:send_transaction, TransactionRequest {
      from: 0x123.into(), to: Some(0x123.into()),
      gas: None, gas_price: Some(0x1.into()),
      value: Some(0x1.into()), data: None,
      nonce: None, condition: None,
    }
    =>
    "eth_sendTransaction", vec![r#"{"from":"0x0000000000000000000000000000000000000123","gasPrice":"0x1","to":"0x0000000000000000000000000000000000000123","value":"0x1"}"#];
    Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()) => 0x123
  );

    rpc_test! (
    Eth:sign, 0x123, Bytes(vec![1, 2, 3, 4])
    =>
    "eth_sign", vec![r#""0x0000000000000000000000000000000000000123""#, r#""0x01020304""#];
    Value::String("0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000123".into()) => 0x123
  );

    rpc_test! (
    Eth:submit_hashrate, 0x123, 0x456
    =>
    "eth_submitHashrate", vec![r#""0x123""#, r#""0x0000000000000000000000000000000000000000000000000000000000000456""#];
    Value::Bool(true) => true
  );

    rpc_test! (
    Eth:submit_work, 0x123, 0x456, 0x789
    =>
    "eth_submitWork", vec![r#""0x0000000000000123""#, r#""0x0000000000000000000000000000000000000000000000000000000000000456""#, r#""0x0000000000000000000000000000000000000000000000000000000000000789""#];
    Value::Bool(true) => true
  );

    rpc_test! (
    Eth:syncing:syncing => "eth_syncing";
    json!({"startingBlock": "0x384","currentBlock": "0x386","highestBlock": "0x454"}) => SyncState::Syncing(SyncInfo { starting_block: 0x384.into(), current_block: 0x386.into(), highest_block: 0x454.into()})
  );

    rpc_test! {
      Eth:syncing:not_syncing => "eth_syncing";
      Value::Bool(false) => SyncState::NotSyncing
    }
}
