use std::time::Duration;
use futures::{Future, Stream};
use api::{Eth, EthFilter, Namespace};
use types::{H256, U256, TransactionRequest};
use {Transport, Error};

const POLL_INTERVAL: u64 = 1;
const REQUIRED_CONFIRMATIONS: usize = 12;

pub fn wait_for_confirmations<'a, T, F, V>(transport: T, poll_interval: Duration, confirmations: usize, check: V)
  -> Box<Future<Item = (), Error = Error> + 'a> where
  T: 'a + Transport + Clone,
  F: 'a + Future<Item = Option<U256>, Error = Error>,
  V: 'a + Fn() -> F,
{
  let eth = EthFilter::new(transport.clone());
  let result = eth.create_blocks_filter()
    .and_then(move |filter| {
      filter.stream(poll_interval)
        .skip(confirmations as u64)
        .and_then(move |_| check())
        .filter_map(|o| o)
        .and_then(move |confirmed_block_number| {
          Eth::new(transport.clone()).block_number()
            .map(move |last_block_number| u64::from(confirmed_block_number) + confirmations as u64 >= u64::from(last_block_number))
        })
        .filter(|confirmed| *confirmed)
        .take(1)
        .collect()
        .map(|_| ())
    });
  Box::new(result)
}

pub fn send_transaction_with_confirmation<'a, T>(transport: T, tx: TransactionRequest, poll_interval: Duration, confirmations: usize) -> Box<Future<Item = H256, Error = Error> + 'a> where
  T: 'a + Transport + Clone {
  let eth = Eth::new(transport.clone());
  let result = eth.send_transaction(tx)
    .and_then(move |hash| {
      wait_for_confirmations(transport.clone(), poll_interval, confirmations, move || {
        let eth = Eth::new(transport.clone());
        eth.transaction_receipt(hash.clone()).map(|option| option.map(|receipt| receipt.block_number))
      })
      .map(move |_| hash)
    });
  Box::new(result)
}

#[cfg(test)]
mod tests {
  use std::time::Duration;
  use futures::Future;
  use helpers::tests::TestTransport;
  use types::{TransactionRequest, TransactionReceipt};
  use super::send_transaction_with_confirmation;
  use rpc::Value;

  #[test]
  fn test_send_transaction_with_confirmation() {
    let mut transport = TestTransport::default();
    let duration = Duration::from_secs(0);
    let confirmations = 3;
    let transaction_request = TransactionRequest {
      from: 0x123.into(),
      to: Some(0x123.into()),
      gas: None,
      gas_price: Some(1.into()),
      value: Some(1.into()),
      data: None,
      nonce: None,
      min_block: None,
    };
    let poll_interval = Duration::from_secs(0);
    transport.add_response(Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000111"#.into()));
    transport.add_response(Value::String("0x123".into()));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into()),
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000457"#.into()),
    ]));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000458"#.into()),
    ]));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000459"#.into()),
    ]));
    transport.add_response(Value::Null);
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000460"#.into()),
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000461"#.into()),
    ]));
    transport.add_response(Value::Null);
    transport.add_response(json!(TransactionReceipt {
      hash: 0.into(),
      index: 0.into(),
      block_hash: 0.into(),
      block_number: 2.into(),
      cumulative_gas_used: 0.into(),
      gas_used: 0.into(),
      contract_address: None,
      logs: vec![],
    }));
    transport.add_response(Value::String("0x5".into()));

    let confirmation = {
      let future = send_transaction_with_confirmation(&transport, transaction_request, poll_interval, confirmations);
      future.wait()
    };

    transport.assert_request("eth_sendTransaction", &[r#"{"from":"0x0000000000000000000000000000000000000123","gasPrice":"0x1","to":"0x0000000000000000000000000000000000000123","value":"0x1"}"#.into()]);
    transport.assert_request("eth_newBlockFilter", &[]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_getTransactionReceipt", &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_getTransactionReceipt", &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()]);
    transport.assert_request("eth_getTransactionReceipt", &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()]);
    assert_eq!(confirmation, Ok(0x111.into()));
  }
}
