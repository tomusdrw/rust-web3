use std::marker::PhantomData;

use rpc;
use futures::{Async, Poll, Future};
use serde;
use serde_json;
use {Error};

/// Value-decoder future.
/// Takes any type which is deserializable from rpc::Value,
/// a future which yields that type, and
pub struct CallResult<T, F> {
  inner: F,
  _marker: PhantomData<T>,
}

impl<T, F> CallResult<T, F> {
  /// Create a new CallResult wrapping the inner future.
  pub fn new(inner: F) -> Self {
    CallResult { inner: inner, _marker: PhantomData }
  }
}

impl<T: serde::de::DeserializeOwned, F> Future for CallResult<T, F>
  where F: Future<Item=rpc::Value, Error=Error>
{
  type Item = T;
  type Error = Error;

  fn poll(&mut self) -> Poll<T, Error> {
    match self.inner.poll() {
      Ok(Async::Ready(x)) => serde_json::from_value(x).map(Async::Ready).map_err(Into::into),
      Ok(Async::NotReady) => Ok(Async::NotReady),
      Err(e) => Err(e),
    }
  }
}

pub fn serialize<T: serde::Serialize>(t: &T) -> rpc::Value {
  serde_json::to_value(t).expect("Types never fail to serialize.")
}

pub fn build_request(id: usize, method: &str, params: Vec<rpc::Value>) -> String {
  let request = rpc::Request::Single(rpc::Call::MethodCall(rpc::MethodCall {
    jsonrpc: Some(rpc::Version::V2),
    method: method.into(),
    params: Some(rpc::Params::Array(params)),
    id: rpc::Id::Num(id as u64),
  }));
  serde_json::to_string(&request).expect("String serialization never fails.")
}

pub fn to_response_from_slice(response: &[u8]) -> Result<rpc::Response, Error> {
  serde_json::from_slice(response)
    .map_err(|e| Error::InvalidResponse(format!("{:?}", e)))
}

pub fn to_result_from_output(output: rpc::Output) -> Result<rpc::Value, Error> {
  match output {
    rpc::Output::Success(success) => Ok(success.result),
    rpc::Output::Failure(failure) => Err(Error::Rpc(failure.error)),
  }
}

pub fn to_result(response: &str) -> Result<rpc::Value, Error> {
  let response = serde_json::from_str(response)
    .map_err(|e| Error::InvalidResponse(format!("{:?}", e)))?;

  match response {
    rpc::Response::Single(output) => to_result_from_output(output),
    _ => Err(Error::InvalidResponse("Expected single, got batch.".into())),
  }
}

#[macro_use]
#[cfg(test)]
pub mod tests {
  use serde_json;
  use std::cell::RefCell;
  use std::collections::VecDeque;
  use futures::{self, Future};
  use rpc;
  use {Result, Error, Transport};

  #[derive(Default)]
  pub struct TestTransport {
    asserted: usize,
    requests: RefCell<Vec<(String, Vec<rpc::Value>)>>,
    response: RefCell<VecDeque<rpc::Value>>,
  }

  impl Transport for TestTransport {
    type Out = Result<rpc::Value>;

    fn execute(&self, method: &str, params: Vec<rpc::Value>) -> Result<rpc::Value> {
      self.requests.borrow_mut().push((method.into(), params));
      match self.response.borrow_mut().pop_front() {
        Some(response) => futures::finished(response).boxed(),
        None => futures::failed(Error::Unreachable).boxed(),
      }
    }
  }

  impl TestTransport {
    pub fn set_response(&mut self, value: rpc::Value) {
      *self.response.borrow_mut() = vec![value].into();
    }

    pub fn add_response(&mut self, value: rpc::Value) {
      self.response.borrow_mut().push_back(value);
    }

    pub fn assert_request(&mut self, method: &str, params: &[String]) {
      let idx = self.asserted;
      self.asserted += 1;

      let (m, p) = self.requests.borrow().get(idx).expect("Expected result.").clone();
      assert_eq!(&m, method);
      let p: Vec<String> = p.into_iter().map(|p| serde_json::to_string(&p).unwrap()).collect();
      assert_eq!(p, params);
    }

    pub fn assert_no_more_requests(&mut self) {
      let requests = self.requests.borrow();
      assert_eq!(self.asserted, requests.len(), "Expected no more requests, got: {:?}", &requests[self.asserted..]);
    }
  }

  macro_rules! rpc_test {
    // With parameters
    (
      $namespace: ident : $name: ident : $test_name: ident  $(, $param: expr)+ => $method: expr,  $results: expr;
      $returned: expr => $expected: expr
    ) => {
      #[test]
      fn $test_name() {
        // given
        let mut transport = $crate::helpers::tests::TestTransport::default();
        transport.set_response($returned);
        let result = {
          let eth = $namespace::new(&transport);

          // when
          eth.$name($($param.into(), )+)
        };

        // then
        transport.assert_request($method, &$results.into_iter().map(Into::into).collect::<Vec<_>>());
        transport.assert_no_more_requests();
        assert_eq!(result.wait(), Ok($expected.into()));
      }
    };
    // With parameters (implicit test name)
    (
      $namespace: ident : $name: ident $(, $param: expr)+ => $method: expr,  $results: expr;
      $returned: expr => $expected: expr
    ) => {
      rpc_test! (
        $namespace : $name : $name $(, $param)+ => $method, $results;
        $returned => $expected
      );
    };

    // No params entry point (explicit name)
    (
      $namespace: ident: $name: ident: $test_name: ident => $method: expr;
      $returned: expr => $expected: expr
    ) => {
      #[test]
      fn $test_name() {
        // given
        let mut transport = $crate::helpers::tests::TestTransport::default();
        transport.set_response($returned);
        let result = {
          let eth = $namespace::new(&transport);

          // when
          eth.$name()
        };

        // then
        transport.assert_request($method, &[]);
        transport.assert_no_more_requests();
        assert_eq!(result.wait(), Ok($expected.into()));
      }
    };

    // No params entry point
    (
      $namespace: ident: $name: ident => $method: expr;
      $returned: expr => $expected: expr
    ) => {
      rpc_test! (
        $namespace: $name: $name => $method;
        $returned => $expected
      );
    }
  }
}
