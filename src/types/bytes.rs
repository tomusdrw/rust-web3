use serde::{
    de::{Error, Unexpected, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::fmt;

/// Raw bytes wrapper
#[derive(Clone, Default, PartialEq, Eq, Hash)]
pub struct Bytes(pub Vec<u8>);

impl<T: Into<Vec<u8>>> From<T> for Bytes {
    fn from(data: T) -> Self {
        Bytes(data.into())
    }
}

impl Serialize for Bytes {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serialized = "0x".to_owned();
        serialized.push_str(&hex::encode(&self.0));
        serializer.serialize_str(serialized.as_ref())
    }
}

impl<'a> Deserialize<'a> for Bytes {
    fn deserialize<D>(deserializer: D) -> Result<Bytes, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_identifier(BytesVisitor)
    }
}

impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let serialized = format!("0x{}", hex::encode(&self.0));
        f.debug_tuple("Bytes").field(&serialized).finish()
    }
}

struct BytesVisitor;

impl<'a> Visitor<'a> for BytesVisitor {
    type Value = Bytes;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a 0x-prefixed hex-encoded vector of bytes")
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: Error,
    {
        if let Some(value) = value.strip_prefix("0x") {
            let bytes = hex::decode(value).map_err(|e| Error::custom(format!("Invalid hex: {}", e)))?;
            Ok(Bytes(bytes))
        } else {
            Err(Error::invalid_value(Unexpected::Str(value), &"0x prefix"))
        }
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: Error,
    {
        self.visit_str(value.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize() {
        assert_eq!(serde_json::from_str::<Bytes>(r#""0x00""#).unwrap(), Bytes(vec![0x00]));
        assert_eq!(
            serde_json::from_str::<Bytes>(r#""0x0123456789AaBbCcDdEeFf""#).unwrap(),
            Bytes(vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF])
        );
        assert_eq!(serde_json::from_str::<Bytes>(r#""0x""#).unwrap(), Bytes(vec![]));

        assert!(serde_json::from_str::<Bytes>("0").is_err(), "Not a string");
        assert!(serde_json::from_str::<Bytes>(r#""""#).is_err(), "Empty string");
        assert!(serde_json::from_str::<Bytes>(r#""0xZZ""#).is_err(), "Invalid hex");
        assert!(
            serde_json::from_str::<Bytes>(r#""deadbeef""#).is_err(),
            "Missing 0x prefix"
        );
        assert!(serde_json::from_str::<Bytes>(r#""数字""#).is_err(), "Non-ASCII");
        assert!(serde_json::from_str::<Bytes>(r#""0x数字""#).is_err(), "Non-ASCII");
    }
}
