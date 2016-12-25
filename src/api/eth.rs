//! `Eth` namespace

use futures::Future;

use helpers;
use types::{Address, BlockNumber};
use {Result, Transport};

/// List of methods from `eth` namespace
pub trait EthApi {
  /// Get list of available accounts.
  fn accounts(&self) -> Result<Vec<Address>>;

  /// Get current block number
  fn block_number(&self) -> Result<BlockNumber>;
}

/// `Eth` namespace
pub struct Eth<'a, T: 'a> {
  transport: &'a T,
}

impl<'a, T: Transport + 'a> Eth<'a, T> {
  /// New `Eth` namespace with given transport.
  pub fn new(transport: &'a T) -> Self {
    Eth {
      transport: transport,
    }
  }
}

impl<'a, T: Transport + 'a> EthApi for Eth<'a, T> {
  fn accounts(&self) -> Result<Vec<Address>> {
    self.transport.execute("eth_accounts", None)
      .and_then(helpers::to_vector)
      .boxed()
  }

  fn block_number(&self) -> Result<BlockNumber> {
    unimplemented!()
  }
}

#[cfg(test)]
mod tests {
  use std::cell::RefCell;
  use super::{Eth, EthApi};
  use futures::{self, Future};
  use rpc;
  use {Result, Error, Transport};

  #[derive(Default)]
  struct TestTransport {
    asserted: usize,
    requests: RefCell<Vec<(String, Option<Vec<String>>)>>,
  }

  impl TestTransport {
    fn assert_request(&mut self, method: &str, params: Option<Vec<String>>) {
      let idx = self.asserted;
      self.asserted += 1;

      let (m, p) = self.requests.borrow().get(idx).expect("Expected result.").clone();
      assert_eq!(&m, method);
      assert_eq!(p, params);
    }

    fn assert_no_more_requests(&mut self) {
      let requests = self.requests.borrow();
      assert_eq!(self.asserted, requests.len(), "Expected no more requests, got: {:?}", &requests[self.asserted..]);
    }
  }

  impl Transport for TestTransport {
    fn execute(&self, method: &str, params: Option<Vec<String>>) -> Result<rpc::Value> {
      self.requests.borrow_mut().push((method.into(), params));
      futures::failed(Error::Unreachable).boxed()
    }
  }

  #[test]
  fn accounts() {
    // given
    let mut transport = TestTransport::default();
    let result = {
      let eth = Eth::new(&transport);

      // when
      eth.accounts()
    };

    // then
    transport.assert_request("eth_accounts", None);
    transport.assert_no_more_requests();
    assert_eq!(result.wait(), Err(Error::Unreachable));
  }
}
