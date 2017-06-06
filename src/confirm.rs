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

pub fn send_transaction_and_wait_for_confirmation<'a, T>(transport: T, tx: TransactionRequest) -> Box<Future<Item = H256, Error = Error> + 'a> where
  T: 'a + Transport + Clone {
  let eth = Eth::new(transport.clone());
  let result = eth.send_transaction(tx)
    .and_then(move |hash| {
      wait_for_confirmations(transport.clone(), Duration::from_secs(POLL_INTERVAL), REQUIRED_CONFIRMATIONS, move || {
        let eth = Eth::new(transport.clone());
        eth.transaction_receipt(hash.clone()).map(|option| option.map(|receipt| receipt.block_number))
      })
      .map(move |_| hash)
    });
  Box::new(result)
}
