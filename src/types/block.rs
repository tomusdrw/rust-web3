use serde::{Serialize, Serializer};
use types::H256;

#[derive(Clone, Debug, PartialEq)]
pub enum BlockNumber {
  Latest,
  Earliest,
  Pending,
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

#[derive(Clone, Debug, PartialEq)]
pub enum BlockId {
  Hash(H256),
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
