//! Contract Functions Output types.

use arrayvec::ArrayVec;
use ethabi::Token;
use contract::Error;
use types::{self, Address, H256, U256, U64};

/// Output type possible to deserialize from Contract ABI
pub trait Detokenize {
  /// Creates a new instance from parsed ABI tokens.
  fn from_tokens(tokens: Vec<Token>) -> Result<Self, Error> where Self: Sized;
}

impl<T: Tokenizable> Detokenize for T {
  fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, Error> {
    if tokens.len() != 1 {
      return Err(Error::InvalidOutputType(format!("Expected single element, got a list: {:?}", tokens)));
    }
    Self::from_token(tokens.drain(..).next().expect("At least one element in vector; qed"))
  }
}

macro_rules! impl_output {
  ($num: expr, $( $ty: ident , )+) => {
    impl<$($ty, )+> Detokenize for ($($ty,)+) where
      $(
        $ty: Tokenizable,
      )+
    {
      fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, Error> {
        if tokens.len() != $num {
          return Err(Error::InvalidOutputType(format!(
            "Expected {} elements, got a list of {}: {:?}",
            $num,
            tokens.len(),
            tokens
          )));
        }
        let mut it = tokens.drain(..);
        Ok(($(
          $ty::from_token(it.next().expect("All elements are in vector; qed"))?,
        )+))
      }
    }
  }
}

impl_output!(1, A, );
impl_output!(2, A, B, );
impl_output!(3, A, B, C, );
impl_output!(4, A, B, C, D, );
impl_output!(5, A, B, C, D, E, );

/// Tokens conversion trait
pub trait Tokenize {
  /// Convert to list of tokens
  fn into_tokens(self) -> Vec<Token>;
}

impl<'a> Tokenize for &'a [Token] {
  fn into_tokens(self) -> Vec<Token> { self.to_vec() }
}

impl<T: Tokenizable> Tokenize for T {
  fn into_tokens(self) -> Vec<Token> { vec![self.into_token()] }
}

impl Tokenize for () {
  fn into_tokens(self) -> Vec<Token> { vec![] }
}

macro_rules! impl_tokens {
  ($( $ty: ident : $no: tt, )+) => {
    impl<$($ty, )+> Tokenize for ($($ty,)+) where
      $(
        $ty: Tokenizable,
      )+
    {
      fn into_tokens(self) -> Vec<Token> {
        vec![
          $( self.$no.into_token(), )+
        ]
      }
    }
  }
}

impl_tokens!(A:0, );
impl_tokens!(A:0, B:1, );
impl_tokens!(A:0, B:1, C:2, );
impl_tokens!(A:0, B:1, C:2, D:3, );
impl_tokens!(A:0, B:1, C:2, D:3, E:4, );

/// Simplified output type for single value.
pub trait Tokenizable {
  /// Converts a `Token` into expected type.
  fn from_token(token: Token) -> Result<Self, Error> where Self: Sized;
  /// Converts a specified type back into token.
  fn into_token(self) -> Token;
}

impl Tokenizable for Token {
  fn from_token(token: Token) -> Result<Self, Error> {
    Ok(token)
  }
  fn into_token(self) -> Token {
    self
  }
}

impl Tokenizable for String {
  fn from_token(token: Token) -> Result<Self, Error> {
    match token {
      Token::String(s) => Ok(s),
      other => Err(Error::InvalidOutputType(format!("Expected `String`, got {:?}", other))),
    }
  }

  fn into_token(self) -> Token {
    Token::String(self)
  }
}

impl Tokenizable for H256 {
  fn from_token(token: Token) -> Result<Self, Error> {
    match token {
      Token::FixedBytes(mut s) => {
        if s.len() != 32 {
          return Err(Error::InvalidOutputType(format!("Expected `H256`, got {:?}", s)));
        }
        let mut data = [0; 32];
        for (idx, val) in s.drain(..).enumerate() {
          data[idx] = val;
        }
        Ok(H256(data))
      },
      other => Err(Error::InvalidOutputType(format!("Expected `H256`, got {:?}", other))),
    }
  }

  fn into_token(self) -> Token {
    Token::FixedBytes(self.0.to_vec())
  }
}


impl Tokenizable for Address {
  fn from_token(token: Token) -> Result<Self, Error> {
    match token {
      Token::Address(data) => Ok(types::H160(data)),
      other => Err(Error::InvalidOutputType(format!("Expected `Address`, got {:?}", other))),
    }
  }

  fn into_token(self) -> Token {
    Token::Address(self.0)
  }
}

macro_rules! uint_tokenizable {
  ($uint: ident, $name: expr) => {
    impl Tokenizable for $uint {
      fn from_token(token: Token) -> Result<Self, Error> {
        match token {
          Token::Int(data) | Token::Uint(data) => Ok($uint::from(data.as_ref())),
          other => Err(Error::InvalidOutputType(format!("Expected `{}`, got {:?}",  $name, other))),
        }
      }

      fn into_token(self) -> Token {
        let u = U256::from(self.0.as_ref());
        Token::Uint(u.0)
      }
    }
  }
}

uint_tokenizable!(U256, "U256");
uint_tokenizable!(U64, "U64");

impl Tokenizable for u64 {
  fn from_token(token: Token) -> Result<Self, Error> {
    match token {
      Token::Int(data) | Token::Uint(data) => Ok(U256::from(data.as_ref()).low_u64()),
      other => Err(Error::InvalidOutputType(format!("Expected `u64`, got {:?}",  other))),
    }
  }

  fn into_token(self) -> Token {
    let u = U256::from(self);
    Token::Uint(u.0)
  }
}

impl Tokenizable for bool {
  fn from_token(token: Token) -> Result<Self, Error> {
    match token {
      Token::Bool(data) => Ok(data),
      other => Err(Error::InvalidOutputType(format!("Expected `bool`, got {:?}", other))),
    }
  }
  fn into_token(self) -> Token {
    Token::Bool(self)
  }
}

impl Tokenizable for Vec<u8> {
  fn from_token(token: Token) -> Result<Self, Error> {
    match token {
      Token::Bytes(data) => Ok(data),
      Token::FixedBytes(data) => Ok(data),
      other => Err(Error::InvalidOutputType(format!("Expected `bytes`, got {:?}", other))),
    }
  }
  fn into_token(self) -> Token {
    Token::Bytes(self)
  }
}

impl<T: Tokenizable> Tokenizable for Vec<T> {
  fn from_token(token: Token) -> Result<Self, Error> {
    match token {
      Token::FixedArray(tokens) | Token::Array(tokens) => tokens.into_iter().map(Tokenizable::from_token).collect(),
      other => Err(Error::InvalidOutputType(format!("Expected `Array`, got {:?}", other))),
    }
  }

  fn into_token(self) -> Token {
    Token::Array(self.into_iter().map(Tokenizable::into_token).collect())
  }
}

macro_rules! impl_fixed_types {
  ($num: expr) => {
    impl Tokenizable for [u8; $num] {
      fn from_token(token: Token) -> Result<Self, Error> {
        match token {
          Token::FixedBytes(bytes) => {
            if bytes.len() != $num {
              return Err(Error::InvalidOutputType(format!("Expected `FixedBytes({})`, got FixedBytes({})", $num, bytes.len())));
            }

            let mut arr = [0; $num];
            arr.copy_from_slice(&bytes);
            Ok(arr)
          },
          other => Err(Error::InvalidOutputType(format!("Expected `FixedBytes({})`, got {:?}", $num, other))),
        }
      }

      fn into_token(self) -> Token {
        Token::FixedBytes(self.to_vec())
      }
    }

    impl<T: Tokenizable + Clone> Tokenizable for [T; $num] {
      fn from_token(token: Token) -> Result<Self, Error> {
        match token {
          Token::FixedArray(tokens) => {
            if tokens.len() != $num {
              return Err(Error::InvalidOutputType(format!("Expected `FixedArray({})`, got FixedArray({})", $num, tokens.len())));
            }

            let mut arr = ArrayVec::<[T; $num]>::new();
            let mut it = tokens.into_iter().map(T::from_token);
            for _ in 0..$num {
              arr.push(it.next().expect("Length validated in guard; qed")?);
            }
            // Can't use expect here because [T; $num]: Debug is not satisfied.
            match arr.into_inner() {
              Ok(arr) => Ok(arr),
              Err(_) => panic!("All elements inserted so the array is full; qed"),
            }
          },
          other => Err(Error::InvalidOutputType(format!("Expected `FixedArray({})`, got {:?}", $num, other))),
        }
      }

      fn into_token(self) -> Token {
        Token::FixedArray(ArrayVec::from(self).into_iter().map(T::into_token).collect())
      }
    }
  }
}

impl_fixed_types!(1);
impl_fixed_types!(2);
impl_fixed_types!(3);
impl_fixed_types!(4);
impl_fixed_types!(5);
impl_fixed_types!(8);
impl_fixed_types!(16);
impl_fixed_types!(32);
impl_fixed_types!(64);
impl_fixed_types!(128);
impl_fixed_types!(256);
impl_fixed_types!(512);
impl_fixed_types!(1024);

#[cfg(test)]
mod tests {
  use ethabi::Token;
  use super::Detokenize;
  use types::{Address, U256};

  fn output<R: Detokenize>() -> R {
    unimplemented!()
  }

  #[test]
  #[ignore]
  fn should_be_able_to_compile() {
    let _tokens: Vec<Token> = output();
    let _uint: U256 = output();
    let _address: Address = output();
    let _string: String = output();
    let _bool: bool = output();
    let _bytes: Vec<u8> = output();

    let _pair: (U256, bool) = output();
    let _vec: Vec<U256> = output();
    let _array: [U256; 4] = output();
    let _bytes: Vec<[[u8; 1]; 64]> = output();

    let _mixed: (Vec<Vec<u8>>, [U256; 4], Vec<U256>, U256) = output();
  }

  #[test]
  fn should_decode_array_of_fixed_bytes() {
    // byte[8][]
    let tokens = vec![
      Token::FixedArray(vec![
        Token::FixedBytes(vec![1]),
        Token::FixedBytes(vec![2]),
        Token::FixedBytes(vec![3]),
        Token::FixedBytes(vec![4]),
        Token::FixedBytes(vec![5]),
        Token::FixedBytes(vec![6]),
        Token::FixedBytes(vec![7]),
        Token::FixedBytes(vec![8]),
      ])
    ];
    let data: [[u8; 1]; 8] = Detokenize::from_tokens(tokens).unwrap();
    assert_eq!(data[0][0], 1);
    assert_eq!(data[1][0], 2);
    assert_eq!(data[2][0], 3);
    assert_eq!(data[7][0], 8);
  }
}
