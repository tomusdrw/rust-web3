//! Ethereum Contract Interface

use ethabi;

use api::Eth;
use contract::result::QueryResult;
use contract::tokens::{Detokenize, Tokenize};
use types::{Address, Bytes, CallRequest, H256, TransactionRequest, TransactionCondition, U256};
use {Transport, Error as ApiError};

mod result;
pub mod tokens;

/// Contract call/query error.
#[derive(Debug)]
pub enum Error {
  /// API call errror.
  Api(ApiError),
  /// ABI encoding error.
  Abi(ethabi::Error),
  /// Invalid output type requested from caller.
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

/// Contract Call/Query Options
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Options {
  /// Fixed gas limit
  pub gas: Option<U256>,
  /// Fixed gas price
  pub gas_price: Option<U256>,
  /// Value to transfer
  pub value: Option<U256>,
  /// Fixed transaction nonce
  pub nonce: Option<U256>,
  /// A conditon to satisfy before including transaction.
  pub condition: Option<TransactionCondition>,
}

impl Options {
  /// Create new default `Options` object with some modifications.
  pub fn with<F>(func: F) -> Options where
    F: FnOnce(&mut Options)
  {
      let mut options = Options::default();
      func(&mut options);
      options
  }
}

/// Ethereum Contract Interface
pub struct Contract<T: Transport> {
  address: Address,
  eth: Eth<T>,
  abi: ethabi::Contract,
}

impl<T: Transport> Contract<T> {
  /// Creates new Contract Interface given blockchain address and ABI
  pub fn new(eth: Eth<T>, address: Address, abi: ethabi::Contract) -> Self {
    Contract {
      address: address,
      eth: eth,
      abi: abi,
    }
  }

  /// Creates new Contract Interface given blockchain address and JSON containing ABI
  pub fn from_json(eth: Eth<T>, address: Address, json: &[u8]) -> Result<Self, ethabi::spec::Error> {
    let abi = ethabi::Contract::new(ethabi::Interface::load(json)?);
    Ok(Self::new(eth, address, abi))
  }

  /// Execute a contract function
  pub fn call<P>(&self, func: &str, params: P, from: Address, options: Options) -> QueryResult<H256, T::Out> where
    P: Tokenize,
  {
    self.abi.function(func.into())
      .and_then(|function| function.encode_call(params.into_tokens()))
      .map(move |data| {
        let result = self.eth.send_transaction(TransactionRequest {
          from: from,
          to: Some(self.address.clone()),
          gas: options.gas,
          gas_price: options.gas_price,
          value: options.value,
          nonce: options.nonce,
          data: Some(Bytes(data)),
          condition: options.condition,
        });
        QueryResult::simple(result)
      })
      .unwrap_or_else(Into::into)
  }

  /// Estimate gas required for this function call.
  pub fn estimate_gas<P>(&self, func: &str, params: P, from: Address, options: Options) -> QueryResult<U256, T::Out> where
    P: Tokenize,
  {
    self.abi.function(func.into())
      .and_then(|function| function.encode_call(params.into_tokens()))
      .map(|data| {
        let result = self.eth.estimate_gas(CallRequest {
          from: Some(from),
          to: self.address.clone(),
          gas: options.gas,
          gas_price: options.gas_price,
          value: options.value,
          data: Some(Bytes(data)),
        }, None);
        QueryResult::simple(result)
      })
      .unwrap_or_else(Into::into)
  }

  /// Call constant function
  pub fn query<R, A, P>(&self, func: &str, params: P, from: A, options: Options) -> QueryResult<R, T::Out> where
    R: Detokenize,
    A: Into<Option<Address>>,
    P: Tokenize,
  {
    self.abi.function(func.into())
      .and_then(|function| function.encode_call(params.into_tokens()).map(|call| (call, function)))
      .map(|(call, function)| {
        let result = self.eth.call(CallRequest {
          from: from.into(),
          to: self.address.clone(),
          gas: options.gas,
          gas_price: options.gas_price,
          value: options.value,
          data: Some(Bytes(call))
        }, None);
        QueryResult::new(result, function)
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
  use types::{Address, H256, U256};
  use {Transport};
  use super::{Contract, Options};

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
      token.query("name", (), None, Options::default()).wait().unwrap()
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
  fn should_query_with_params() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(rpc::Value::String("0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000c48656c6c6f20576f726c64210000000000000000000000000000000000000000".into()));

    let result: String = {
      let token = contract(&transport);

      // when
      token.query("name", (), Address::from(5), Options::with(|mut options| {
        options.gas_price = Some(10_000_000.into());
      })).wait().unwrap()
    };

    // then
    transport.assert_request("eth_call", &[
      "{\"data\":\"0x06fdde03\",\"from\":\"0x0000000000000000000000000000000000000005\",\"gasPrice\":\"0x989680\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(),
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
      token.call("name", (), 5.into(), Options::default()).wait().unwrap()
    };

    // then
    transport.assert_request("eth_sendTransaction", &[
      "{\"data\":\"0x06fdde03\",\"from\":\"0x0000000000000000000000000000000000000005\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(),
    ]);
    transport.assert_no_more_requests();
    assert_eq!(result, 5.into());
  }

  #[test]
  fn should_estimate_gas_usage() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(rpc::Value::String(format!("{:?}", U256::from(5))));

    let result = {
      let token = contract(&transport);

      // when
      token.estimate_gas("name", (), 5.into(), Options::default()).wait().unwrap()
    };

    // then
    transport.assert_request("eth_estimateGas", &[
      "{\"data\":\"0x06fdde03\",\"from\":\"0x0000000000000000000000000000000000000005\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(),
      "\"latest\"".into(),
    ]);
    transport.assert_no_more_requests();
    assert_eq!(result, 5.into());
  }

  #[test]
  fn should_query_single_parameter_function() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(rpc::Value::String("0x0000000000000000000000000000000000000000000000000000000000000020".into()));

    let result: U256 = {
      let token = contract(&transport);

      // when
      token.query("balanceOf", (Address::from(5)), None, Options::default()).wait().unwrap()
    };

    // then
    transport.assert_request("eth_call", &[
      "{\"data\":\"0x70a082310000000000000000000000000000000000000000000000000000000000000005\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(),
      "\"latest\"".into(),
    ]);
    transport.assert_no_more_requests();
    assert_eq!(result, 0x20.into());
  }
}

