use ethabi;
use futures::{Future, task::Poll};
use serde;
use std::mem;

use crate::contract;
use crate::contract::tokens::Detokenize;
use crate::error;
use crate::helpers;
use crate::rpc;
use crate::types::Bytes;

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
    F: Future<Output = error::Result<rpc::Value>>,
{
    type Output = Result<T, contract::Error>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let ResultType::Decodable(ref mut inner, ref function) = self.inner {
            let bytes: Bytes = ready!(inner.poll());
            return Ok(Poll::Ready(T::from_tokens(function.decode_output(&bytes.0)?)?));
        }

        match mem::replace(&mut self.inner, ResultType::Done) {
            ResultType::Constant(res) => res.map(Poll::Ready),
            _ => panic!("Unsupported state"),
        }
    }
}

impl<T: serde::de::DeserializeOwned, F> Future for CallFuture<T, F>
where
    F: Future<Output = error::Result<rpc::Value>>,
{
    type Output = Result<T, contract::Error>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        if let ResultType::Simple(ref mut inner) = self.inner {
            let hash: T = ready!(inner.poll());
            return Ok(Poll::Ready(hash));
        }

        match mem::replace(&mut self.inner, ResultType::Done) {
            ResultType::Constant(res) => res.map(Poll::Ready),
            _ => panic!("Unsupported state"),
        }
    }
}
