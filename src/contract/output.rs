//! Contract Functions Output types.

use ethabi;
use contract::Error;
use types::{H256, U256};

/// Output type possible to deserialize from Contract ABI
pub trait Output {
  /// Creates a new instance from parsed ABI tokens.
  fn from_tokens(tokens: Vec<ethabi::Token>) -> Result<Self, Error> where Self: Sized;
}

impl Output for Vec<ethabi::Token> {
  fn from_tokens(tokens: Vec<ethabi::Token>) -> Result<Self, Error> {
    Ok(tokens)
  }
}

/// Simplified output type for single value.
pub trait SingleOutput: Output {
  /// Converts a `Token` into expected type.
  fn from_token(token: ethabi::Token) -> Result<Self, Error> where Self: Sized;
}

impl<T: SingleOutput> Output for T {
  fn from_tokens(mut tokens: Vec<ethabi::Token>) -> Result<Self, Error> {
    if tokens.len() != 1 {
      return Err(Error::InvalidOutputType(format!("Expected single element, got a list: {:?}", tokens)));
    }

    Self::from_token(tokens.drain(..).next().expect("At least one element in vector; qed"))
  }
}

impl SingleOutput for String {
  fn from_token(token: ethabi::Token) -> Result<Self, Error> {
    match token {
      ethabi::Token::String(s) => Ok(s),
      other => Err(Error::InvalidOutputType(format!("Expected `String`, got {:?}", other))),
    }
  }
}

impl SingleOutput for H256 {
  fn from_token(token: ethabi::Token) -> Result<Self, Error> {
    match token {
      ethabi::Token::FixedBytes(mut s) => {
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
}


impl SingleOutput for U256 {
  fn from_token(token: ethabi::Token) -> Result<Self, Error> {
    match token {
      ethabi::Token::Uint(data) => Ok(U256(data)),
      other => Err(Error::InvalidOutputType(format!("Expected `U256`, got {:?}", other))),
    }
  }
}

