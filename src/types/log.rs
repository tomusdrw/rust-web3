use types::{Bytes, U256, H256, Index};

/// A log produced by a transaction.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Log {
  /// Log index in block.
  #[serde(rename="logIndex")]
  pub log_index: Index,
  /// Block number this log occurred in.
  #[serde(rename="blockNumber")]
  pub block_number: U256,
  /// Hash of block this log occurred in.
  #[serde(rename="blockHash")]
  pub block_hash: H256,
  /// Transaction this log occurred in.
  #[serde(rename="transactionHash")]
  pub transaction_hash: H256,
  /// Index of this log within the transaction.
  #[serde(rename="transactionIndex")]
  pub transaction_index: Index,
  /// Log data.
  pub data: Bytes,
  /// List of topics this log contains.
  pub topics: Vec<H256>,
}
