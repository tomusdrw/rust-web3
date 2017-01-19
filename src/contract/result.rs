use std::marker::PhantomData;
use std::mem;
use ethabi;
use futures::{Future, Async, Poll};
use serde;

use contract;
use contract::tokens::Output;
use helpers;
use rpc;
use types::Bytes;
use {Error as ApiError};

enum ResultType<T, F> {
  Decodable(helpers::CallResult<Bytes, F>, ethabi::Function),
  Simple(helpers::CallResult<T, F>),
  Constant(Result<T, contract::Error>),
  Done,
}

/// Function-specific bytes-decoder future.
/// Takes any type which is deserializable from `Vec<ethabi::Token>`,
/// a function definition and a future which yields that type.
pub struct QueryResult<T, F> where
  F: Future<Item=rpc::Value, Error=ApiError>,
{
  inner: ResultType<T, F>,
  _marker: PhantomData<T>,
}

impl<T, F, E> From<E> for QueryResult<T, F> where
  F: Future<Item=rpc::Value, Error=ApiError>,
  E: Into<contract::Error>,
{
  fn from(e: E) -> Self {
    QueryResult::constant(Err(e.into()))
  }
}

impl<T, F> QueryResult<T, F> where
  F: Future<Item=rpc::Value, Error=ApiError>,
{
  /// Create a new `QueryResult` wrapping the inner future.
  pub fn new(inner: helpers::CallResult<Bytes, F>, function: ethabi::Function) -> Self {
    QueryResult {
      inner: ResultType::Decodable(inner, function),
      _marker: PhantomData,
    }
  }

  pub fn simple(inner: helpers::CallResult<T, F>) -> Self {
    QueryResult {
      inner: ResultType::Simple(inner),
      _marker: PhantomData,
    }
  }

  fn constant(result: Result<T, contract::Error>) -> Self {
    QueryResult {
      inner: ResultType::Constant(result),
      _marker: PhantomData,
    }
  }
}

impl<T: Output + serde::Deserialize, F> Future for QueryResult<T, F> where
  F: Future<Item=rpc::Value, Error=ApiError>,
{
  type Item = T;
  type Error = contract::Error;

  fn poll(&mut self) -> Poll<T, contract::Error> {
    let result = match self.inner {
      ResultType::Simple(ref mut inner) => {
        let hash: T = try_ready!(inner.poll());
        Some(Ok(hash))
      },
      ResultType::Decodable(ref mut inner, ref function) => {
        let bytes: Bytes = try_ready!(inner.poll());
        Some(T::from_tokens(function.decode_output(bytes.0)?))
      },
      _ => None,
    };

    if let Some(res) = result {
      return res.map(Async::Ready).map_err(Into::into);
    }

    match mem::replace(&mut self.inner, ResultType::Done) {
      ResultType::Constant(Ok(res)) => Ok(Async::Ready(res)),
      ResultType::Constant(Err(err)) => Err(err),
      _ => panic!("Unexpected state!"),
    }
  }
}

