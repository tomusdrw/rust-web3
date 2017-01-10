//! Contract Functions Output types.

use ethabi::Token;
use contract::Error;
use types::{self, Address, H256, U256};

/// Output type possible to deserialize from Contract ABI
pub trait Output {
  /// Creates a new instance from parsed ABI tokens.
  fn from_tokens(tokens: Vec<Token>) -> Result<Self, Error> where Self: Sized;
}

impl Output for Vec<Token> {
  fn from_tokens(tokens: Vec<Token>) -> Result<Self, Error> {
    Ok(tokens)
  }
}

/// Tokens conversion trait
pub trait Tokens {
  /// Convert to list of tokens
  fn into_tokens(self) -> Vec<Token>;
}

impl<'a> Tokens for &'a [Token] {
  fn into_tokens(self) -> Vec<Token> { self.to_vec() }
}

impl<T: Tokenizable> Tokens for T {
  fn into_tokens(self) -> Vec<Token> { vec![self.into_token()] }
}

impl Tokens for () {
  fn into_tokens(self) -> Vec<Token> { vec![] }
}

macro_rules! impl_tokens {
  ($( $ty: ident : $no: tt, )+) => {
    impl<$($ty, )+> Tokens for ($($ty,)+) where
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

impl<T: Tokenizable> Output for T {
  fn from_tokens(mut tokens: Vec<Token>) -> Result<Self, Error> {
    if tokens.len() != 1 {
      return Err(Error::InvalidOutputType(format!("Expected single element, got a list: {:?}", tokens)));
    }
    Self::from_token(tokens.drain(..).next().expect("At least one element in vector; qed"))
  }
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

impl Tokenizable for U256 {
  fn from_token(token: Token) -> Result<Self, Error> {
    match token {
      Token::Uint(data) => Ok(U256(data)),
      other => Err(Error::InvalidOutputType(format!("Expected `U256`, got {:?}", other))),
    }
  }

  fn into_token(self) -> Token {
    Token::Uint(self.0)
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
      other => Err(Error::InvalidOutputType(format!("Expected `bool`, got {:?}", other))),
    }
  }
  fn into_token(self) -> Token {
    Token::Bytes(self)
  }
}
