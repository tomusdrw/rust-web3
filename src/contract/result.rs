use ethabi;
use futures::{Async, Future, Poll};
use serde;
use std::mem;

use crate::contract;
use crate::contract::tokens::Detokenize;
use crate::helpers;
use crate::rpc;
use crate::types::Bytes;
use crate::Error as ApiError;

#[derive(Debug)]
enum ResultType<T, F> {
    Decodable(helpers::CallFuture<Bytes, F>, ethabi::Function),
    Simple(helpers::CallFuture<T, F>),
    Constant(Result<T, contract::Error>),
    Done,
}

/// A standard function (RPC) call result.
/// Takes any type which is deserializable from JSON,
/// a function definition and a future which yields that type.
#[derive(Debug)]
pub struct CallFuture<T, F> {
    inner: ResultType<T, F>,
}

impl<T, F> From<crate::helpers::CallFuture<T, F>> for CallFuture<T, F> {
    fn from(inner: crate::helpers::CallFuture<T, F>) -> Self {
        CallFuture {
            inner: ResultType::Simple(inner),
        }
    }
}

impl<T, F, E> From<E> for CallFuture<T, F>
where
    E: Into<contract::Error>,
{
    fn from(e: E) -> Self {
        CallFuture {
            inner: ResultType::Constant(Err(e.into())),
        }
    }
}

/// Function-specific bytes-decoder future.
/// Takes any type which is deserializable from `Vec<ethabi::Token>`,
/// a function definition and a future which yields that type.
#[derive(Debug)]
pub struct QueryResult<T, F> {
    inner: ResultType<T, F>,
}

impl<T, F, E> From<E> for QueryResult<T, F>
where
    E: Into<contract::Error>,
{
    fn from(e: E) -> Self {
        QueryResult {
            inner: ResultType::Constant(Err(e.into())),
        }
    }
}

impl<T, F> QueryResult<T, F> {
    /// Create a new `QueryResult` wrapping the inner future.
    pub fn new(inner: helpers::CallFuture<Bytes, F>, function: ethabi::Function) -> Self {
        QueryResult {
            inner: ResultType::Decodable(inner, function),
        }
    }
}

impl<T: Detokenize, F> Future for QueryResult<T, F>
where
    F: Future<Item = rpc::Value, Error = ApiError>,
{
    type Item = T;
    type Error = contract::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let ResultType::Decodable(ref mut inner, ref function) = self.inner {
            let bytes: Bytes = try_ready!(inner.poll());
            return Ok(Async::Ready(T::from_tokens(function.decode_output(&bytes.0)?)?));
        }

        match mem::replace(&mut self.inner, ResultType::Done) {
            ResultType::Constant(res) => res.map(Async::Ready),
            _ => panic!("Unsupported state"),
        }
    }
}

impl<T: serde::de::DeserializeOwned, F> Future for CallFuture<T, F>
where
    F: Future<Item = rpc::Value, Error = ApiError>,
{
    type Item = T;
    type Error = contract::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let ResultType::Simple(ref mut inner) = self.inner {
            let hash: T = try_ready!(inner.poll());
            return Ok(Async::Ready(hash));
        }

        match mem::replace(&mut self.inner, ResultType::Done) {
            ResultType::Constant(res) => res.map(Async::Ready),
            _ => panic!("Unsupported state"),
        }
    }
}
