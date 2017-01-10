use serde::{Serialize, Serializer};
use types::H256;

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
