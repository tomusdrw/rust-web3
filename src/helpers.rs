use rpc;
use {Error};

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
  }

  impl Transport for TestTransport {
    fn execute(&self, method: &str, params: Option<Vec<String>>) -> Result<rpc::Value> {
      self.requests.borrow_mut().push((method.into(), params));
      futures::failed(Error::Unreachable).boxed()
    }
  }

  impl TestTransport {
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

  macro_rules! rpc_test_wo_params {
    (
      $namespace: ident: $name: ident => $method: expr
    ) => {
      #[test]
      fn $name() {
        // given
        let mut transport = $crate::helpers::tests::TestTransport::default();
        let result = {
          let eth = $namespace::new(&transport);

          // when
          eth.$name()
        };

        // then
        transport.assert_request($method, None);
        transport.assert_no_more_requests();
        assert_eq!(result.wait(), Err(Error::Unreachable));
      }
    }
  }
}
