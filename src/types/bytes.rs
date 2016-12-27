use serde::{Serialize, Serializer, Deserialize, Deserializer, Error};
use serde::de::Visitor;
use rustc_serialize::hex::{FromHex, ToHex};

#[derive(Clone, Debug, PartialEq)]
pub struct Bytes(pub Vec<u8>);

impl Serialize for Bytes {
	fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
	where S: Serializer {
		let mut serialized = "0x".to_owned();
		serialized.push_str(self.0.to_hex().as_ref());
		serializer.serialize_str(serialized.as_ref())
	}
}

impl Deserialize for Bytes {
	fn deserialize<D>(deserializer: &mut D) -> Result<Bytes, D::Error>
	where D: Deserializer {
		deserializer.deserialize(BytesVisitor)
	}
}

struct BytesVisitor;

impl Visitor for BytesVisitor {
	type Value = Bytes;

	fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E> where E: Error {
    if value.len() >= 2 && &value[0..2] == "0x" && value.len() & 1 == 0 {
			Ok(Bytes(FromHex::from_hex(&value[2..]).map_err(|_| Error::custom("invalid hex"))?))
		} else {
			Err(Error::custom("invalid format"))
		}
	}

	fn visit_string<E>(&mut self, value: String) -> Result<Self::Value, E> where E: Error {
		self.visit_str(value.as_ref())
	}
}

