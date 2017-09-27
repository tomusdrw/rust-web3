use std::mem;
use ethabi;
use futures::{Future, Async, Poll};
use serde;

use contract;
use contract::tokens::Detokenize;
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

/// A standard function (RPC) call result.
/// Takes any type which is deserializable from JSON,
/// a function definition and a future which yields that type.
pub struct CallResult<T, F> {
  inner: ResultType<T, F>,
}

impl<T, F> From<::helpers::CallResult<T, F>> for CallResult<T, F> {
  fn from(inner: ::helpers::CallResult<T, F>) -> Self {
    CallResult {
      inner: ResultType::Simple(inner),
    }
  }
}

impl<T, F, E> From<E> for CallResult<T, F> where
  E: Into<contract::Error>,
{
  fn from(e: E) -> Self {
    CallResult {
      inner: ResultType::Constant(Err(e.into())),
    }
  }
}

/// Function-specific bytes-decoder future.
/// Takes any type which is deserializable from `Vec<ethabi::Token>`,
/// a function definition and a future which yields that type.
pub struct QueryResult<T, F> {
  inner: ResultType<T, F>,
}

impl<T, F, E> From<E> for QueryResult<T, F> where
  E: Into<contract::Error>,
{
  fn from(e: E) -> Self {
    QueryResult {
      inner: ResultType::Constant(Err(e.into()))
    }
  }
}

impl<T, F> QueryResult<T, F> {
  /// Create a new `QueryResult` wrapping the inner future.
  pub fn new(inner: helpers::CallResult<Bytes, F>, function: ethabi::Function) -> Self {
    QueryResult {
      inner: ResultType::Decodable(inner, function),
    }
  }
}

impl<T: Detokenize, F> Future for QueryResult<T, F> where
  F: Future<Item=rpc::Value, Error=ApiError>,
{
  type Item = T;
  type Error = contract::Error;

  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    if let ResultType::Decodable(ref mut inner, ref function) = self.inner {
      let bytes: Bytes = try_ready!(inner.poll());
      return Ok(Async::Ready(T::from_tokens(function.decode_output(&bytes.0)?)?))
    }

    match mem::replace(&mut self.inner, ResultType::Done) {
      ResultType::Constant(res) => res.map(Async::Ready),
      _ => panic!("Unsupported state"),
    }
  }
}

impl<T: serde::de::DeserializeOwned, F> Future for CallResult<T, F> where
  F: Future<Item=rpc::Value, Error=ApiError>,
{
  type Item = T;
  type Error = contract::Error;

  fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
    if let ResultType::Simple(ref mut inner) = self.inner {
      let hash: T = try_ready!(inner.poll());
      return Ok(Async::Ready(hash))
    }

    match mem::replace(&mut self.inner, ResultType::Done) {
      ResultType::Constant(res) => res.map(Async::Ready),
      _ => panic!("Unsupported state"),
    }
  }
}

