use std::marker::PhantomData;
use std::mem;
use ethabi;
use futures::{Future, Async, Poll};

use contract;
use contract::output::Output;
use helpers;
use rpc;
use types::{Bytes, H256};
use {Error as ApiError};

enum ResultType<T, F> {
  Call(helpers::CallResult<Bytes, F>, ethabi::Function),
  Hash(helpers::CallResult<H256, F>),
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
      inner: ResultType::Call(inner, function),
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

impl<F> QueryResult<H256, F> where
  F: Future<Item=rpc::Value, Error=ApiError>,
{
  /// Create a new `QueryResult` for transaction hash: `H256`.
  pub fn for_hash(hash: helpers::CallResult<H256, F>) -> Self {
    QueryResult {
      inner: ResultType::Hash(hash),
      _marker: PhantomData,
    }
  }
}

impl<T: Output, F> Future for QueryResult<T, F> where
  F: Future<Item=rpc::Value, Error=ApiError>,
{
  type Item = T;
  type Error = contract::Error;

  fn poll(&mut self) -> Poll<T, contract::Error> {
    match self.inner {
      ResultType::Hash(ref mut inner) => {
        let result: Poll<H256, _> = inner.poll();
        return match result {
          Ok(Async::Ready(hash)) => T::from_tokens(vec![ethabi::Token::FixedBytes(hash.0.to_vec())])
            .map(Async::Ready)
            .map_err(Into::into),
          Ok(Async::NotReady) => Ok(Async::NotReady),
          Err(e) => Err(e.into()),
        };
      },
      ResultType::Call(ref mut inner, ref function) => {
        let result: Poll<Bytes, _> = inner.poll();
        return match result {
          Ok(Async::Ready(x)) => T::from_tokens(function.decode_output(x.0)?)
            .map(Async::Ready)
            .map_err(Into::into),
          Ok(Async::NotReady) => Ok(Async::NotReady),
          Err(e) => Err(e.into()),
        };
      },
      _ => {},
    }

    match mem::replace(&mut self.inner, ResultType::Done) {
      ResultType::Constant(Ok(res)) => Ok(Async::Ready(res)),
      ResultType::Constant(Err(err)) => Err(err),
      _ => panic!("Unexpected state!"),
    }
  }
}

