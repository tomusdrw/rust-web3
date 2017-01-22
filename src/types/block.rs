use serde::{Serialize, Serializer, Deserialize, Deserializer};
use types::{Bytes, Transaction, U64, U256, H256, H160, H2048};

/// Block transactions: either hashes or full transactions,
/// based on flag in request.
#[derive(Debug, Clone, PartialEq)]
pub enum BlockTransactions {
  /// Full transactions.
  Full(Vec<Transaction>),
  /// Transaction hashes.
  Hashes(Vec<H256>),
  /// No transactions.
  None,
}

impl BlockTransactions {
  /// Convert to `Option<Vec<Transaction>>`. Some on `Full` variant, None otherwise.
  pub fn full(self) -> Option<Vec<Transaction>> {
    match self {
      BlockTransactions::Full(txs) => Some(txs),
      _ => None,
    }
  }

  /// Convert to `Option<Vec<Transaction>>`. Some on `Hashes` variant, None otherwise.
  pub fn hashes(self) -> Option<Vec<H256>> {
    match self {
      BlockTransactions::Hashes(txs) => Some(txs),
      _ => None,
    }
  }
}

impl Deserialize for BlockTransactions {
  fn deserialize<D>(deserializer: &mut D) -> Result<Self, D::Error>
    where D: Deserializer
  {
    let res = Vec::<Transaction>::deserialize(deserializer).map(BlockTransactions::Full)
      .or_else(|_| Vec::<H256>::deserialize(deserializer).map(BlockTransactions::Hashes));
    match res {
      Ok(BlockTransactions::Full(ref v)) if v.is_empty() => Ok(BlockTransactions::None),
      Ok(BlockTransactions::Hashes(ref v)) if v.is_empty() => Ok(BlockTransactions::None),
      other => other,
    }
  }
}

/// The block type returned from RPC calls.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Block {
  /// Hash of the block
	pub hash: Option<H256>,
	/// Hash of the parent
	#[serde(rename="parentHash")]
	pub parent_hash: H256,
	/// Hash of the uncles
	#[serde(rename="sha3Uncles")]
	pub uncles_hash: H256,
  /// Miner/author's address.
  #[serde(rename="miner")]
  pub author: H160,
	/// State root hash
	#[serde(rename="stateRoot")]
	pub state_root: H256,
	/// Transactions root hash
	#[serde(rename="transactionsRoot")]
	pub transactions_root: H256,
	/// Transactions receipts root hash
	#[serde(rename="receiptsRoot")]
	pub receipts_root: H256,
	/// Block number. Null if pending.
	pub number: Option<U64>,
	/// Gas Used
	#[serde(rename="gasUsed")]
	pub gas_used: U256,
	/// Gas Limit
	#[serde(rename="gasLimit")]
	pub gas_limit: U256,
	/// Extra data
	#[serde(rename="extraData")]
	pub extra_data: Bytes,
	/// Logs bloom
	#[serde(rename="logsBloom")]
	pub logs_bloom: H2048,
	/// Timestamp
	pub timestamp: U256,
	/// Difficulty
	pub difficulty: U256,
	/// Total difficulty
	#[serde(rename="totalDifficulty")]
	pub total_difficulty: U256,
	/// Seal fields
	#[serde(rename="sealFields")]
	pub seal_fields: Vec<Bytes>,
	/// Uncles' hashes
	pub uncles: Vec<H256>,
	/// Transactions
	pub transactions: BlockTransactions,
	/// Size in bytes
	pub size: Option<U256>,
}

/// Block Number
#[derive(Clone, Debug, PartialEq)]
pub enum BlockNumber {
  /// Latest block
  Latest,
  /// Earliest block (genesis)
  Earliest,
  /// Pending block (not yet part of the blockchain)
  Pending,
  /// Block by number from canon chain
  Number(u64),
}

impl From<u64> for BlockNumber {
  fn from(num: u64) -> Self {
    BlockNumber::Number(num)
  }
}

impl Serialize for BlockNumber {
	fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
		match *self {
			BlockNumber::Number(ref x) => serializer.serialize_str(&format!("0x{:x}", x)),
			BlockNumber::Latest => serializer.serialize_str("latest"),
			BlockNumber::Earliest => serializer.serialize_str("earliest"),
			BlockNumber::Pending => serializer.serialize_str("pending"),
		}
	}
}

/// Block Identifier
#[derive(Clone, Debug, PartialEq)]
pub enum BlockId {
  /// By Hash
  Hash(H256),
  /// By Number
  Number(BlockNumber),
}

impl Serialize for BlockId {
	fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
		match *self {
			BlockId::Hash(ref x) => serializer.serialize_str(&format!("0x{}", x)),
			BlockId::Number(ref num) => num.serialize(serializer),
		}
	}
}

impl From<u64> for BlockId {
  fn from(num: u64) -> Self {
    BlockNumber::Number(num).into()
  }
}

impl From<BlockNumber> for BlockId {
  fn from(num: BlockNumber) -> Self {
    BlockId::Number(num)
  }
}

impl From<H256> for BlockId {
  fn from(hash: H256) -> Self {
    BlockId::Hash(hash)
  }
}
