use ethabi;

use api::Eth;
use contract::helpers::QueryResult;
use contract::output::Output;
use types::{Address, Bytes, CallRequest, H256, TransactionRequest, U256};
use {Transport, Error as ApiError};

#[derive(Debug)]
pub enum Error {
  Api(ApiError),
  Abi(ethabi::Error),
  InvalidOutputType(String),
}

impl From<ethabi::Error> for Error {
  fn from(error: ethabi::Error) -> Self {
    Error::Abi(error)
  }
}
impl From<ApiError> for Error {
  fn from(error: ApiError) -> Self {
    Error::Api(error)
  }
}

pub struct Contract<T: Transport> {
  address: Address,
  eth: Eth<T>,
  abi: ethabi::Contract,
}

impl<T: Transport> Contract<T> {
  pub fn new(eth: Eth<T>, address: Address, abi: ethabi::Contract) -> Self {
    Contract {
      address: address,
      eth: eth,
      abi: abi,
    }
  }

  pub fn from_json(eth: Eth<T>, address: Address, json: &[u8]) -> Result<Self, ethabi::spec::Error> {
    let abi = ethabi::Contract::new(ethabi::Interface::load(json)?);
    Ok(Self::new(eth, address, abi))
  }

  /// Call constant function with no parameters
  pub fn query0<O: Output>(&self, func: &str, from: Option<Address>) -> QueryResult<O, T::Out> {
    self.abi.function(func.into()).and_then(|function| {
      function.encode_call(vec![]).map(|call| (call, function))
    }).map(|(call, function)| {
      let result = self.eth.call(CallRequest {
        from: from,
        to: self.address.clone(),
        gas: None,
        gas_price: None,
        value: None,
        data: Some(Bytes(call))
      }, None);
      QueryResult::new(result, function)
    }).unwrap_or_else(Into::into)
  }

  /// Call function with no parameters
  pub fn call0(&self, func: &str, from: Address, value: Option<U256>) -> QueryResult<H256, T::Out> {
    // TODO [ToDr] Estimate Gas?
    self.abi.function(func.into())
      .and_then(|function| function.encode_call(vec![]))
      .map(|data| {
        let result = self.eth.send_transaction(TransactionRequest {
          from: from,
          to: Some(self.address.clone()),
          gas: None,
          gas_price: None,
          value: value,
          nonce: None,
          data: Some(Bytes(data)),
          min_block: None,
        });
        QueryResult::for_hash(result)
      })
      .unwrap_or_else(Into::into)
  }
}

#[cfg(test)]
mod tests {
  use api::{self, Namespace};
  use futures::Future;
  use helpers::tests::TestTransport;
  use rpc;
  use types::H256;
  use {Transport};
  use super::Contract;

  fn contract<T: Transport>(transport: &T) -> Contract<&T> {
    let eth = api::Eth::new(transport);
    Contract::from_json(eth, 1.into(), include_bytes!("./res/token.json")).unwrap()
  }

  #[test]
  fn should_call_constant_function() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(rpc::Value::String("0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000c48656c6c6f20576f726c64210000000000000000000000000000000000000000".into()));

    let result: String = {
      let token = contract(&transport);

      // when
      token.query0("name", None).wait().unwrap()
    };

    // then
    transport.assert_request("eth_call", &[
      "{\"data\":\"0x06fdde03\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(),
      "\"latest\"".into(),
    ]);
    transport.assert_no_more_requests();
    assert_eq!(result, "Hello World!".to_owned());
  }

  #[test]
  fn should_call_a_contract_function() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(rpc::Value::String(format!("{:?}", H256::from(5))));

    let result = {
      let token = contract(&transport);

      // when
      token.call0("name", 5.into(), None).wait().unwrap()
    };

    // then
    transport.assert_request("eth_sendTransaction", &[
      "{\"data\":\"0x06fdde03\",\"from\":\"0x0000000000000000000000000000000000000005\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(),
    ]);
    transport.assert_no_more_requests();
    assert_eq!(result, 5.into());
  }
}
