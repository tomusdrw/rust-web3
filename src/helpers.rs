use rpc;
use rustc_serialize::hex::FromHex;
use serde;
use serde_json;
use types;
use {Error};

pub fn serialize<T: serde::Serialize>(t: &T) -> String {
  serde_json::to_string(t).expect("Types serialization is never failing.")
}

pub fn to_vector(val: rpc::Value) -> Result<Vec<String>, Error> {
  let invalid = Error::InvalidResponse(format!("Expected vector of strings, got {:?}", val));

  if let rpc::Value::Array(val) = val {
    val.into_iter().map(|v| match v {
     rpc::Value::String(s) => Ok(s),
      _ => Err(invalid.clone()),
    }).collect()
  } else {
    Err(invalid)
  }
}

pub fn to_u256(val: rpc::Value) -> Result<types::U256, Error> {
  // TODO [ToDr] proper type
  to_string(val)
}

pub fn to_h256(val: rpc::Value) -> Result<types::H256, Error> {
  // TODO [ToDr] proper type
  to_string(val)
}

pub fn to_u256_option(val: rpc::Value) -> Result<Option<types::U256>, Error> {
  Ok(if val == rpc::Value::Null {
    None
  } else {
    Some(try!(to_u256(val)))
  })
}

pub fn to_address(val: rpc::Value) -> Result<types::Address, Error> {
  // TODO [ToDr] proper type
  to_string(val)
}

pub fn to_string(val: rpc::Value) -> Result<String, Error> {
  if let rpc::Value::String(s) = val {
    Ok(s)
  } else {
    Err(Error::InvalidResponse(format!("Expected string, got {:?}", val)))
  }
}

pub fn to_bytes(val: rpc::Value) -> Result<types::Bytes, Error> {
  if let rpc::Value::String(s) = val {
    s[2..].from_hex().map(types::Bytes).map_err(|e| Error::InvalidResponse(
      format!("Invalid hex string returned: {:?}", e)      
    ))
  } else {
    Err(Error::InvalidResponse(format!("Expected bytes, got {:?}", val)))
  }
}

pub fn to_bool(val: rpc::Value) -> Result<bool, Error> {
  if let rpc::Value::Bool(b) = val {
    Ok(b)
  } else {
    Err(Error::InvalidResponse(format!("Expected bool, got {:?}", val)))
  }
}

#[macro_use]
#[cfg(test)]
pub mod tests {
  use std::cell::RefCell;
  use futures::{self, Future};
  use rpc;
  use {Result, Error, Transport};

  #[derive(Default)]
  pub struct TestTransport {
    asserted: usize,
    requests: RefCell<Vec<(String, Option<Vec<String>>)>>,
    response: RefCell<Option<rpc::Value>>,
  }

  impl Transport for TestTransport {
    fn execute(&self, method: &str, params: Option<Vec<String>>) -> Result<rpc::Value> {
      self.requests.borrow_mut().push((method.into(), params));
      match self.response.borrow_mut().take() {
        Some(response) => futures::finished(response).boxed(),
        None => futures::failed(Error::Unreachable).boxed(),
      }
    }
  }

  impl TestTransport {
    pub fn set_response(&mut self, value: rpc::Value) {
      *self.response.borrow_mut() = Some(value);
    }

    pub fn assert_request(&mut self, method: &str, params: Option<Vec<String>>) {
      let idx = self.asserted;
      self.asserted += 1;

      let (m, p) = self.requests.borrow().get(idx).expect("Expected result.").clone();
      assert_eq!(&m, method);
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
        transport.assert_request($method, Some($results.into_iter().map(Into::into).collect()));
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

    // No params entry point
    (
      $namespace: ident: $name: ident => $method: expr;
      $returned: expr => $expected: expr
    ) => {
      #[test]
      fn $name() {
        // given
        let mut transport = $crate::helpers::tests::TestTransport::default();
        transport.set_response($returned);
        let result = {
          let eth = $namespace::new(&transport);

          // when
          eth.$name()
        };

        // then
        transport.assert_request($method, None);
        transport.assert_no_more_requests();
        assert_eq!(result.wait(), Ok($expected.into()));
      }
    }
  }
}
