use types::{BlockNumber, Bytes, U256, H160, H256, Index};

/// A log produced by a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

/// Filter
#[derive(Default, Debug, PartialEq, Clone, Serialize)]
pub struct Filter {
	/// From Block
	#[serde(rename="fromBlock")]
	from_block: Option<BlockNumber>,
	/// To Block
	#[serde(rename="toBlock")]
	to_block: Option<BlockNumber>,
	/// Address
	address: Option<Vec<H160>>,
	/// Topics
	topics: Option<Vec<Option<Vec<H256>>>>,
	/// Limit
	limit: Option<usize>,
}

/// Filter Builder
#[derive(Default, Clone)]
pub struct FilterBuilder {
  filter: Filter,
}

impl FilterBuilder {
  /// Sets from block
  pub fn from_block(mut self, block: BlockNumber) -> Self {
    self.filter.from_block = Some(block);
    self
  }

  /// Sets to block
  pub fn to_block(mut self, block: BlockNumber) -> Self {
    self.filter.to_block = Some(block);
    self
  }

  /// Single address
  pub fn address(mut self, address: Vec<H160>) -> Self {
    self.filter.address = Some(address);
    self
  }

  /// Topics
  pub fn topics(
    mut self,
    topic1: Option<Vec<H256>>,
    topic2: Option<Vec<H256>>,
    topic3: Option<Vec<H256>>,
    topic4: Option<Vec<H256>>,
  ) -> Self {
    self.filter.topics = Some(vec![topic1, topic2, topic3, topic4]);
    self
  }

  /// Limit the result
  pub fn limit(mut self, limit: usize) -> Self {
    self.filter.limit = Some(limit);
    self
  }

  /// Returns filter
  pub fn build(&self) -> Filter {
    self.filter.clone()
  }
}
