use std::str::FromStr;
use std::fmt;
use serde;

const PREFIX: usize = 2;
const LEN: usize = 32;

/// Uint serialization.
#[derive(Default, Clone, Copy, PartialEq, Hash)]
pub struct U256([u8; LEN]);

impl Eq for U256 { }

impl From<u64> for U256 {
  fn from(mut num: u64) -> Self {
    let mut arr = [0; LEN];
    for i in 0..8 {
      arr[LEN - 1 - i] =  num as u8;
      num = num >> 8;
    }
    U256(arr)
  }
}

// TODO [ToDr] Get rid of this implementation in favour of `.parse()`
impl<'a> From<&'a str> for U256 {
  fn from(string: &'a str) -> Self {
    string.parse().expect("From<&str> is deprecated. Use `.parse()` instead to handle possible errors.")
  }
}

#[derive(Debug)]
pub enum FromStrError {
  InvalidLength,
  InvalidPrefix,
  InvalidCharacter(char),
}

impl FromStr for U256 {
  type Err = FromStrError;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let len = s.len();
    if len < PREFIX || len > LEN + PREFIX {
      return Err(FromStrError::InvalidLength);
    }

    if &s[0..PREFIX] != "0x" {
      return Err(FromStrError::InvalidPrefix);
    }

    let mut arr = [0; LEN];
    for (idx, byte) in s[PREFIX..].bytes().rev().enumerate() {
      let byte = match byte {
        b'A'...b'F' => byte - b'A' + 10,
        b'a'...b'f' => byte - b'a' + 10,
        b'0'...b'9' => byte - b'0',
        _ => return Err(FromStrError::InvalidCharacter(byte as char)),
      } as u8;

      let pos = idx >> 1;
      let shift = idx - (pos << 1);
      println!("{} {} {}", idx, pos, shift);
      arr[LEN - 1 - pos] |= byte << (shift * 4);
    }

    Ok(U256(arr))
  }
}

impl fmt::Display for U256 {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    // TODO [ToDr] Decimal?
    write!(f, "0x")?;
    fmt::LowerHex::fmt(self, f)
  }
}

impl fmt::Debug for U256 {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "0x")?;
    fmt::LowerHex::fmt(self, f)
  }
}

impl fmt::LowerHex for U256 {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    let mut skiping = true;

    for i in 0..LEN {
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

impl serde::Serialize for U256 {
  fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error> where S: serde::Serializer {
    serializer.serialize_str(&format!("0x{:x}", self))
  }
}

impl serde::Deserialize for U256 {
  fn deserialize<D>(deserializer: &mut D) -> Result<U256, D::Error>
    where D: serde::Deserializer {
      struct UintVisitor;

      impl serde::de::Visitor for UintVisitor {
        type Value = U256;

        fn visit_str<E>(&mut self, value: &str) -> Result<Self::Value, E> where E: serde::Error {
          value.parse().map_err(|e| serde::Error::custom(format!("Invalid hex value: {:?}", e)))
        }

        fn visit_string<E>(&mut self, value: String) -> Result<Self::Value, E> where E: serde::Error {
          self.visit_str(&value)
        }
      }

      deserializer.deserialize(UintVisitor)
    }
}


#[cfg(test)]
mod tests {
  use super::U256;
  use serde_json;

  type Res = Result<U256, serde_json::Error>;

  #[test]
  fn should_display_correctly() {
    let mut arr = [0; 32];
    arr[31] = 0;
    arr[30] = 15;
    arr[29] = 1;
    arr[28] = 0;
    arr[27] = 10;
    let a = U256(arr);
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
