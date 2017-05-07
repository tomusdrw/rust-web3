use std::hash::{Hash, Hasher};
use std::str::FromStr;
use std::{cmp, fmt};
use serde;

const PREFIX: usize = 2;

#[derive(Debug)]
pub enum FromStrError {
  InvalidLength { got: usize, expected: usize },
  InvalidPrefix,
  InvalidCharacter(char),
}

macro_rules! impl_uint {
  ($name: ident, $len: expr) => {
    impl_uint!($name, $len, false);

    impl fmt::Display for $name {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO [ToDr] Decimal?
        write!(f, "0x")?;
        fmt::LowerHex::fmt(self, f)
      }
    }
  };

  (hash => $name: ident, $len: expr) => {
    impl_uint!($name, $len, true);

    impl fmt::Display for $name {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x")?;
        for i in &self.0[0..2] {
            write!(f, "{:02x}", i)?;
        }
        write!(f, "…")?;
        for i in &self.0[$len - 2..$len] {
            write!(f, "{:02x}", i)?;
        }
        Ok(())
      }
    }
  };

  ($name: ident, $len: expr, $strict: expr) => {
    /// Uint serialization.
    pub struct $name(pub [u8; $len]);

    impl Default for $name {
      fn default() -> Self {
        $name([0; $len])
      }
    }

    impl Eq for $name { }

    impl PartialEq for $name {
      fn eq(&self, other: &$name) -> bool {
        for i in 0..$len {
          if self.0[i] != other.0[i] {
            return false;
          }
        }
        true
      }
    }

    impl Ord for $name {
      fn cmp(&self, other: &Self) -> cmp::Ordering {
        for i in 0..$len {
          if self.0[i] < other.0[i] {
            return cmp::Ordering::Less;
          }

          if self.0[i] > other.0[i] {
            return cmp::Ordering::Greater;
          }
        }

        cmp::Ordering::Equal
      }
    }

    impl PartialOrd for $name {
      fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
      }
    }

    impl Copy for $name {}

    impl Clone for $name {
      fn clone(&self) -> Self {
        let mut v = $name::default();
        v.0.copy_from_slice(&self.0);
        v
      }
    }

    impl Hash for $name {
      fn hash<H: Hasher>(&self, state: &mut H) {
          self.0.hash(state);
      }
    }

    impl From<u64> for $name {
      fn from(mut num: u64) -> Self {
        let mut arr = [0; $len];
        for i in 0..8 {
          arr[$len - 1 - i] =  num as u8;
          num = num >> 8;
        }
        $name(arr)
      }
    }

    impl<'a> From<&'a [u8]> for $name {
      fn from(x: &'a [u8]) -> Self {
        let mut arr = [0; $len];
        let len = cmp::min(x.len(), $len);
        arr[$len - len .. ].copy_from_slice(&x[x.len() - $len .. ]);
        $name(arr)
      }
    }

    impl FromStr for $name {
      type Err = FromStrError;

      fn from_str(s: &str) -> Result<Self, Self::Err> {
        let strict_len = $strict;
        let len = s.len();
        let expected = $len * 2 + PREFIX;
        if len < PREFIX || len > expected || (strict_len && (len < expected)) {
          return Err(FromStrError::InvalidLength { got: len, expected: expected });
        }

        if &s[0..PREFIX] != "0x" {
          return Err(FromStrError::InvalidPrefix);
        }

        let mut arr = [0; $len];
        for (idx, byte) in s[PREFIX..].bytes().rev().enumerate() {
          let byte = match byte {
            b'A'...b'F' => byte - b'A' + 10,
            b'a'...b'f' => byte - b'a' + 10,
            b'0'...b'9' => byte - b'0',
            _ => return Err(FromStrError::InvalidCharacter(byte as char)),
          } as u8;

          let pos = idx >> 1;
          let shift = idx - (pos << 1);
          arr[$len - 1 - pos] |= byte << (shift * 4);
        }

        Ok($name(arr))
      }
    }

    impl fmt::Debug for $name {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "0x")?;
        fmt::LowerHex::fmt(self, f)
      }
    }

    impl fmt::LowerHex for $name {
      fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut skiping = !$strict;
        for i in 0..$len {
          match self.0[i] {
            0 if skiping => {},
            _ if skiping => {
              skiping = false;
              write!(f, "{:x}", self.0[i])?;
            },
            _ => {
              skiping = false;
              write!(f, "{:02x}", self.0[i])?;
            }
          }
        }

        if skiping {
          write!(f, "0")?;
        }

        Ok(())
      }
    }

    impl serde::Serialize for $name {
      fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
        serializer.serialize_str(&format!("0x{:x}", self))
      }
    }

    impl serde::Deserialize for $name {
      fn deserialize<D>(deserializer: D) -> Result<$name, D::Error>
        where D: serde::Deserializer {
          struct UintVisitor;

          impl serde::de::Visitor for UintVisitor {
            type Value = $name;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
              write!(formatter, "a 0x-prefixed hex-encoded number")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> where E: serde::de::Error {
              value.parse().map_err(|e| E::custom(format!("Invalid hex value: {:?}", e)))
            }

            fn visit_string<E>(self, value: String) -> Result<Self::Value, E> where E: serde::de::Error {
              self.visit_str(&value)
            }
          }

          deserializer.deserialize(UintVisitor)
        }
    }
  };
}

impl_uint!(U64, 8);
impl_uint!(U256, 32);

impl_uint!(hash => H64, 8);
impl_uint!(hash => H128, 16);
impl_uint!(hash => H160, 20);
impl_uint!(hash => H256, 32);
impl_uint!(hash => H512, 64);
impl_uint!(hash => H2048, 256);

#[cfg(test)]
mod tests {
  use super::{H128, U256};
  use serde_json;

  type Res = Result<U256, serde_json::Error>;

  #[test]
  fn should_compare_correctly() {
    let mut arr = [0u8; 32];
    arr[31] = 0;
    arr[30] = 15;
    arr[29] = 1;
    arr[28] = 0;
    arr[27] = 10;
    let a = U256::from(arr.as_ref());
    arr[27] = 9;
    let b = U256::from(arr.as_ref());
    let c = U256::from(0);
    let d = U256::from(10_000);

    assert!(b < a);
    assert!(d < a);
    assert!(d < b);
    assert!(c < a);
    assert!(c < b);
    assert!(c < d);
  }

  #[test]
  fn should_display_correctly() {
    let mut arr = [0u8; 32];
    arr[31] = 0;
    arr[30] = 15;
    arr[29] = 1;
    arr[28] = 0;
    arr[27] = 10;
    let a = U256::from(arr.as_ref());
    let b = U256::from(1023);
    let c = U256::from(0);
    let d = U256::from(10000);

    // Debug
    assert_eq!(&format!("{:?}", a), "0xa00010f00");
    assert_eq!(&format!("{:?}", b), "0x3ff");
    assert_eq!(&format!("{:?}", c), "0x0");
    assert_eq!(&format!("{:?}", d), "0x2710");

    // Display
    assert_eq!(&format!("{}", a), "0xa00010f00");
    assert_eq!(&format!("{}", b), "0x3ff");
    assert_eq!(&format!("{}", c), "0x0");
    assert_eq!(&format!("{}", d), "0x2710");

    // Lowerhex
    assert_eq!(&format!("{:x}", a), "a00010f00");
    assert_eq!(&format!("{:x}", b), "3ff");
    assert_eq!(&format!("{:x}", c), "0");
    assert_eq!(&format!("{:x}", d), "2710");
  }

  #[test]
  fn should_display_hash_correctly() {
    let mut arr = [0; 16];
    arr[15] = 0;
    arr[14] = 15;
    arr[13] = 1;
    arr[12] = 0;
    arr[11] = 10;
    let a = H128(arr);
    let b = H128::from(1023);
    let c = H128::from(0);
    let d = H128::from(10000);

    // Debug
    assert_eq!(&format!("{:?}", a), "0x00000000000000000000000a00010f00");
    assert_eq!(&format!("{:?}", b), "0x000000000000000000000000000003ff");
    assert_eq!(&format!("{:?}", c), "0x00000000000000000000000000000000");
    assert_eq!(&format!("{:?}", d), "0x00000000000000000000000000002710");

    // Display
    assert_eq!(&format!("{}", a), "0x0000…0f00");
    assert_eq!(&format!("{}", b), "0x0000…03ff");
    assert_eq!(&format!("{}", c), "0x0000…0000");
    assert_eq!(&format!("{}", d), "0x0000…2710");

    // Lowerhex
    assert_eq!(&format!("{:x}", a), "00000000000000000000000a00010f00");
    assert_eq!(&format!("{:x}", b), "000000000000000000000000000003ff");
    assert_eq!(&format!("{:x}", c), "00000000000000000000000000000000");
    assert_eq!(&format!("{:x}", d), "00000000000000000000000000002710");
  }

  #[test]
  fn should_deserialize_hash_correctly() {
    let deserialized1: H128 = serde_json::from_str(r#""0x00000000000000000000000a00010f00""#).unwrap();

    assert_eq!(deserialized1, 0xa00010f00.into());
  }

  #[test]
  fn should_serialize_u256() {
    let serialized1 = serde_json::to_string(&U256::from(0)).unwrap();
    let serialized2 = serde_json::to_string(&U256::from(1)).unwrap();
    let serialized3 = serde_json::to_string(&U256::from(16)).unwrap();
    let serialized4 = serde_json::to_string(&U256::from(256)).unwrap();

    assert_eq!(serialized1, r#""0x0""#);
    assert_eq!(serialized2, r#""0x1""#);
    assert_eq!(serialized3, r#""0x10""#);
    assert_eq!(serialized4, r#""0x100""#);
  }

  #[test]
  fn should_fail_to_deserialize_decimals() {
    let deserialized1: Res = serde_json::from_str(r#""""#);
    let deserialized2: Res = serde_json::from_str(r#""0""#);
    let deserialized3: Res = serde_json::from_str(r#""10""#);
    let deserialized4: Res = serde_json::from_str(r#""1000000""#);
    let deserialized5: Res = serde_json::from_str(r#""1000000000000000000""#);

    assert!(deserialized1.is_err());
    assert!(deserialized2.is_err());
    assert!(deserialized3.is_err());
    assert!(deserialized4.is_err());
    assert!(deserialized5.is_err());
  }

  #[test]
  fn should_deserialize_u256() {
    let deserialized1: U256 = serde_json::from_str(r#""0x""#).unwrap();
    let deserialized2: U256 = serde_json::from_str(r#""0x0""#).unwrap();
    let deserialized3: U256 = serde_json::from_str(r#""0x1""#).unwrap();
    let deserialized4: U256 = serde_json::from_str(r#""0x01""#).unwrap();
    let deserialized5: U256 = serde_json::from_str(r#""0x100""#).unwrap();

    assert_eq!(deserialized1, U256([0; 32]));
    assert_eq!(deserialized2, 0.into());
    assert_eq!(deserialized3, 1.into());
    assert_eq!(deserialized4, 1.into());
    assert_eq!(deserialized5, 256.into());
  }
}
