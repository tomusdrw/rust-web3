//! Web3 helpers.

use std::marker::PhantomData;

use rpc;
use futures::{Async, Future, Poll};
use serde;
use serde_json;
use {Error, ErrorKind};

/// Value-decoder future.
/// Takes any type which is deserializable from rpc::Value and a future which yields that
/// type, and yields the deserialized value
#[derive(Debug)]
pub struct CallFuture<T, F> {
    inner: F,
    _marker: PhantomData<T>,
}

impl<T, F> CallFuture<T, F> {
    /// Create a new CallFuture wrapping the inner future.
    pub fn new(inner: F) -> Self {
        CallFuture {
            inner: inner,
            _marker: PhantomData,
        }
    }
}

impl<T: serde::de::DeserializeOwned, F> Future for CallFuture<T, F>
where
    F: Future<Item = rpc::Value, Error = Error>,
{
    type Item = T;
    type Error = Error;

    fn poll(&mut self) -> Poll<T, Error> {
        match self.inner.poll() {
            Ok(Async::Ready(x)) => serde_json::from_value(x)
                .map(Async::Ready)
                .map_err(Into::into),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}

/// Serialize a type. Panics if the type is returns error during serialization.
pub fn serialize<T: serde::Serialize>(t: &T) -> rpc::Value {
    serde_json::to_value(t).expect("Types never fail to serialize.")
}

/// Serializes a request to string. Panics if the type returns error during serialization.
pub fn to_string<T: serde::Serialize>(request: &T) -> String {
    serde_json::to_string(&request).expect("String serialization never fails.")
}

/// Build a JSON-RPC request.
pub fn build_request(id: usize, method: &str, params: Vec<rpc::Value>) -> rpc::Call {
    rpc::Call::MethodCall(rpc::MethodCall {
        jsonrpc: Some(rpc::Version::V2),
        method: method.into(),
        params: Some(rpc::Params::Array(params)),
        id: rpc::Id::Num(id as u64),
    })
}

/// Parse bytes slice into JSON-RPC response.
pub fn to_response_from_slice(response: &[u8]) -> Result<rpc::Response, Error> {
    serde_json::from_slice(response).map_err(|e| ErrorKind::InvalidResponse(format!("{:?}", e)).into())
}

/// Parse bytes slice into JSON-RPC notification.
pub fn to_notification_from_slice(notification: &[u8]) -> Result<rpc::Notification, Error> {
    serde_json::from_slice(notification).map_err(|e| ErrorKind::InvalidResponse(format!("{:?}", e)).into())
}

/// Parse a Vec of `rpc::Output` into `Result`.
pub fn to_results_from_outputs(outputs: Vec<rpc::Output>) -> Result<Vec<Result<rpc::Value, Error>>, Error> {
    Ok(outputs.into_iter().map(to_result_from_output).collect())
}

/// Parse `rpc::Output` into `Result`.
pub fn to_result_from_output(output: rpc::Output) -> Result<rpc::Value, Error> {
    match output {
        rpc::Output::Success(success) => Ok(success.result),
        rpc::Output::Failure(failure) => Err(ErrorKind::Rpc(failure.error).into()),
    }
}

#[macro_use]
#[cfg(test)]
pub mod tests {
    use serde_json;
    use std::cell::RefCell;
    use std::collections::VecDeque;
    use std::rc::Rc;
    use futures;
    use rpc;
    use {ErrorKind, RequestId, Result, Transport};

    #[derive(Debug, Default, Clone)]
    pub struct TestTransport {
        asserted: usize,
        requests: Rc<RefCell<Vec<(String, Vec<rpc::Value>)>>>,
        responses: Rc<RefCell<VecDeque<rpc::Value>>>,
    }

    impl Transport for TestTransport {
        type Out = Result<rpc::Value>;

        fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
            let request = super::build_request(1, method, params.clone());
            self.requests.borrow_mut().push((method.into(), params));
            (self.requests.borrow().len(), request)
        }

        fn send(&self, id: RequestId, request: rpc::Call) -> Result<rpc::Value> {
            match self.responses.borrow_mut().pop_front() {
                Some(response) => Box::new(futures::finished(response)),
                None => {
                    println!("Unexpected request (id: {:?}): {:?}", id, request);
                    Box::new(futures::failed(ErrorKind::Unreachable.into()))
                }
            }
        }
    }

    impl TestTransport {
        pub fn set_response(&mut self, value: rpc::Value) {
            *self.responses.borrow_mut() = vec![value].into();
        }

        pub fn add_response(&mut self, value: rpc::Value) {
            self.responses.borrow_mut().push_back(value);
        }

        pub fn assert_request(&mut self, method: &str, params: &[String]) {
            let idx = self.asserted;
            self.asserted += 1;

            let (m, p) = self.requests
                .borrow()
                .get(idx)
                .expect("Expected result.")
                .clone();
            assert_eq!(&m, method);
            let p: Vec<String> = p.into_iter()
                .map(|p| serde_json::to_string(&p).unwrap())
                .collect();
            assert_eq!(p, params);
        }

        pub fn assert_no_more_requests(&mut self) {
            let requests = self.requests.borrow();
            assert_eq!(
                self.asserted,
                requests.len(),
                "Expected no more requests, got: {:?}",
                &requests[self.asserted..]
            );
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
