use serde::{Serialize, Serializer};

#[derive(Clone, Debug, PartialEq)]
pub enum BlockId {
  Latest,
  Earliest,
  Pending,
  Number(u64),
}

impl Serialize for BlockId {
	fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
		match *self {
			BlockId::Number(ref x) => serializer.serialize_str(&format!("0x{:x}", x)),
			BlockId::Latest => serializer.serialize_str("latest"),
			BlockId::Earliest => serializer.serialize_str("earliest"),
			BlockId::Pending => serializer.serialize_str("pending"),
		}
	}
}
