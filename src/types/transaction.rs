use types::{H160, H256, U256, Index, Log, Bytes};

/// Description of a Transaction, pending or in the chain.
#[derive(Debug, Default, Clone, PartialEq, Deserialize)]
pub struct Transaction {
  /// Hash
  pub hash: H256,
  /// Nonce
  pub nonce: U256,
  /// Block hash. None when pending.
  #[serde(rename="blockHash")]
  pub block_hash: Option<H256>,
  /// Block number. None when pending.
  #[serde(rename="blockNumber")]
  pub block_number: Option<U256>,
  /// Transaction Index. None when pending.
  #[serde(rename="transactionIndex")]
  pub transaction_index: Option<Index>,
  /// Sender
  pub from: H160,
  /// Recipient (None when contract creation)
  pub to: Option<H160>,
  /// Transfered value
  pub value: U256,
  /// Gas Price
  #[serde(rename="gasPrice")]
  pub gas_price: U256,
  /// Gas amount
  pub gas: U256,
  /// Input data
  pub input: Bytes,
}

/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receipt {
  /// Hash
  pub hash: H256,
  /// Index within the block.
  pub index: Index,
  /// Hash of the block this transaction was included within.
  #[serde(rename="blockHash")]
  pub block_hash: H256,
  /// Number of the block this transaction was included within.
  #[serde(rename="blockNumber")]
  pub block_number: U256,
  /// Cumulative gas used within the block after this was executed.
  #[serde(rename="cumulativeGasUsed")]
  pub cumulative_gas_used: U256,
  /// Gas used by this transaction alone.
  #[serde(rename="gasUsed")]
  pub gas_used: U256,
  /// Contract address created, or `None` if not a deployment.
  #[serde(rename="contractAddress")]
  pub contract_address: Option<H160>,
  /// Logs generated within this transaction.
  pub logs: Vec<Log>,
}
