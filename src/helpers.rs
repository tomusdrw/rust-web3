//! Web3 helpers.

use crate::{error, rpc, Error};
use futures::{
    task::{Context, Poll},
    Future,
};
use pin_project::pin_project;
use std::{marker::PhantomData, pin::Pin};

/// Takes any type which is deserializable from rpc::Value and such a value and
/// yields the deserialized value
pub fn decode<T: serde::de::DeserializeOwned>(value: rpc::Value) -> error::Result<T> {
    serde_json::from_value(value).map_err(Into::into)
}

/// Calls decode on the result of the wrapped future.
#[pin_project]
#[derive(Debug)]
pub struct CallFuture<T, F> {
    #[pin]
    inner: F,
    _marker: PhantomData<T>,
}

impl<T, F> CallFuture<T, F> {
    /// Create a new CallFuture wrapping the inner future.
    pub fn new(inner: F) -> Self {
        CallFuture {
            inner,
            _marker: PhantomData,
        }
    }
}

impl<T, F> Future for CallFuture<T, F>
where
    T: serde::de::DeserializeOwned,
    F: Future<Output = error::Result<rpc::Value>>,
{
    type Output = error::Result<T>;

    fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let this = self.project();
        let x = ready!(this.inner.poll(ctx));
        Poll::Ready(x.and_then(decode))
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
        params: rpc::Params::Array(params),
        id: rpc::Id::Num(id as u64),
    })
}

/// Parse bytes slice into JSON-RPC response.
/// It looks for arbitrary_precision feature as a temporary workaround for https://github.com/tomusdrw/rust-web3/issues/460.
pub fn to_response_from_slice(response: &[u8]) -> error::Result<rpc::Response> {
    if cfg!(feature = "arbitrary_precision") {
        let val: serde_json::Value =
            serde_json::from_slice(response).map_err(|e| Error::InvalidResponse(format!("{:?}", e)))?;
        serde_json::from_value(val).map_err(|e| Error::InvalidResponse(format!("{:?}", e)))
    } else {
        serde_json::from_slice(response).map_err(|e| Error::InvalidResponse(format!("{:?}", e)))
    }
}

/// Parse bytes slice into JSON-RPC notification.
pub fn to_notification_from_slice(notification: &[u8]) -> error::Result<rpc::Notification> {
    serde_json::from_slice(notification).map_err(|e| error::Error::InvalidResponse(format!("{:?}", e)))
}

/// Parse a Vec of `rpc::Output` into `Result`.
pub fn to_results_from_outputs(outputs: Vec<rpc::Output>) -> error::Result<Vec<error::Result<rpc::Value>>> {
    Ok(outputs.into_iter().map(to_result_from_output).collect())
}

/// Parse `rpc::Output` into `Result`.
pub fn to_result_from_output(output: rpc::Output) -> error::Result<rpc::Value> {
    match output {
        rpc::Output::Success(success) => Ok(success.result),
        rpc::Output::Failure(failure) => Err(error::Error::Rpc(failure.error)),
    }
}

#[macro_use]
#[cfg(test)]
pub mod tests {
    macro_rules! rpc_test {
    // With parameters
    (
      $namespace: ident : $name: ident : $test_name: ident  $(, $param: expr)+ => $method: expr,  $results: expr;
      $returned: expr => $expected: expr
    ) => {
      #[test]
      fn $test_name() {
        // given
        let mut transport = $crate::transports::test::TestTransport::default();
        transport.set_response($returned);
        let result = {
          let eth = $namespace::new(&transport);

          // when
          eth.$name($($param.into(), )+)
        };

        // then
        transport.assert_request($method, &$results.into_iter().map(Into::into).collect::<Vec<_>>());
        transport.assert_no_more_requests();
        let result = futures::executor::block_on(result);
        assert_eq!(result, Ok($expected.into()));
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
        let mut transport = $crate::transports::test::TestTransport::default();
        transport.set_response($returned);
        let result = {
          let eth = $namespace::new(&transport);

          // when
          eth.$name()
        };

        // then
        transport.assert_request($method, &[]);
        transport.assert_no_more_requests();
        let result = futures::executor::block_on(result);
        assert_eq!(result, Ok($expected.into()));
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
