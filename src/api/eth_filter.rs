//! `Eth` namespace, filters.


use std::marker::PhantomData;
use std::time::Duration;
use serde::de::DeserializeOwned;
use tokio_timer::Timer;
use futures::{Async, Poll, Future, Stream, stream};

use api::Namespace;
use helpers::{self, CallResult};
use types::{Filter, H256, Log, U256};
use {Transport, Error, rpc};

pub trait FilterInterface {
  type Item;

  fn constructor() -> &'static str;
}

/// Logs Filter
pub struct LogsFilter;

impl FilterInterface for LogsFilter {
  type Item = Log;

  fn constructor() -> &'static str {
    "eth_newFilter"
  }
}

/// New blocks hashes filter.
pub struct BlocksFilter;

impl FilterInterface for BlocksFilter {
  type Item = H256;

  fn constructor() -> &'static str {
    "eth_newBlockFilter"
  }
}

/// New Pending Transactions Filter
pub struct PendingTransactionsFilter;

impl FilterInterface for PendingTransactionsFilter {
  type Item = H256;

  fn constructor() -> &'static str {
    "eth_newPendingTransactionFilter"
  }
}

/// Base filter handle.
/// Uninstall filter on drop.
/// Allows to poll the filter.
pub struct BaseFilter<T: Transport, F: FilterInterface> {
  id: U256,
  transport: T,
  interface: PhantomData<F>,
}

impl<T: Transport, F: FilterInterface> BaseFilter<T, F> {
  /// Polls this filter for changes.
  /// Will return logs that happened after previous poll.
  pub fn poll(&self) -> CallResult<Option<Vec<F::Item>>, T::Out> {
    let id = helpers::serialize(&self.id);
    CallResult::new(self.transport.execute("eth_getFilterChanges", vec![id]))
  }

  pub fn stream<'a>(self, poll_interval: Duration) -> Box<Stream<Item = F::Item, Error = Error> + 'a> where
    T: 'a,
    F: 'static,
    F::Item: DeserializeOwned + 'static,
  {
    let result = Timer::default().interval(poll_interval)
      .map_err(|_| Error::Unreachable)
      .then(move |_| self.poll().map(|optional| optional.unwrap_or_else(Default::default)))
      .map(|res| res.into_iter().map(Ok).collect::<Vec<Result<_, Error>>>())
      .map(stream::iter)
      .flatten();
    Box::new(result)
  }

  /// Uninstalls the filter
  pub fn uninstall(self) -> CallResult<bool, T::Out> where Self: Sized {
    self.uninstall_internal()
  }

  fn uninstall_internal(&self) -> CallResult<bool, T::Out> {
    let id = helpers::serialize(&self.id);
    CallResult::new(self.transport.execute("eth_uninstallFilter", vec![id]))
  }
}

impl<T: Transport> BaseFilter<T, LogsFilter> {
  pub fn logs(&self) -> CallResult<Vec<Log>, T::Out> {
    let id = helpers::serialize(&self.id);
    CallResult::new(self.transport.execute("eth_getFilterLogs", vec![id]))
  }
}

impl<T: Transport, F: FilterInterface> Drop for BaseFilter<T, F> {
  fn drop(&mut self) {
    let _ = self.uninstall_internal().wait();
  }
}

fn create_filter<T: Transport + Clone, F: FilterInterface>(t: T, arg: Vec<rpc::Value>) -> CreateFilter<T, F> {
  CreateFilter {
    transport: t.clone(),
    interface: PhantomData,
    future: CallResult::new(t.execute(F::constructor(), arg))
  }
}

pub struct CreateFilter<T: Transport, F: FilterInterface> {
  transport: T,
  interface: PhantomData<F>,
  future: CallResult<U256, T::Out>,
}

impl<T, F> Future for CreateFilter<T, F> where
  T: Transport + Clone,
  F: FilterInterface
{
  type Item = BaseFilter<T, F>;
  type Error = Error;

  fn poll(&mut self) -> Poll<Self::Item, Error> {
    match self.future.poll() {
      Ok(Async::Ready(x)) => Ok(Async::Ready(BaseFilter {
        id: x,
        transport: self.transport.clone(),
        interface: PhantomData,
      })),
      Ok(Async::NotReady) => Ok(Async::NotReady),
      Err(e) => Err(e),
    }
  }
}

/// `Eth` namespace, filters
pub struct EthFilter<T> {
  transport: T,
}

impl<T: Transport + Clone> Namespace<T> for EthFilter<T> {
  fn new(transport: T) -> Self where Self: Sized {
    EthFilter {
      transport: transport
    }
  }
}

impl<T: Transport + Clone> EthFilter<T> {
  /// Installs a new logs filter.
  pub fn new_logs_filter(&self, filter: Filter) -> CreateFilter<T, LogsFilter> {
    let f = helpers::serialize(&filter);
    create_filter(self.transport.clone(), vec![f])
  }

  /// Installs a new block filter.
  pub fn new_blocks_filter(&self) -> CreateFilter<T, BlocksFilter> {
    create_filter(self.transport.clone(), vec![])
  }

  /// Installs a new pending transactions filter.
  pub fn new_pending_transactions_filter(&self) -> CreateFilter<T, PendingTransactionsFilter> {
    create_filter(self.transport.clone(), vec![])
  }
}

#[cfg(test)]
mod tests {
  use std::time::Duration;
  use futures::{Future, Stream};
  use serde_json;
  use rpc::Value;

  use api::Namespace;
  use helpers::tests::TestTransport;
  use types::{Bytes, Log, FilterBuilder};

  use super::EthFilter;

  #[test]
  fn logs_filter() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    {
      let eth = EthFilter::new(&transport);

      // when
      let filter = FilterBuilder::default().limit(10).build();
      let filter = eth.new_logs_filter(filter).wait().unwrap();
      assert_eq!(filter.id, 0x123.into());
    };

    // then
    transport.assert_request("eth_newFilter", &[
      r#"{"address":null,"fromBlock":null,"limit":10,"toBlock":null,"topics":null}"#.into()
    ]);
    transport.assert_request("eth_uninstallFilter", &[r#""0x123""#.into()]);
    transport.assert_no_more_requests();
  }

  #[test]
  fn logs_filter_get_logs() {
    // given
    let log = Log {
      address: 1.into(),
      topics: vec![],
      data: Bytes(vec![]),
      block_hash: Some(2.into()),
      block_number: Some(1.into()),
      transaction_hash: Some(3.into()),
      transaction_index: Some(0.into()),
      log_index: Some(0.into()),
      transaction_log_index: Some(0.into()),
      log_type: "mined".to_owned(),
    };

    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    transport.add_response(Value::Array(vec![
      serde_json::to_value(&log).unwrap(),
    ]));
    let result = {
      let eth = EthFilter::new(&transport);

      // when
      let filter = FilterBuilder::default().topics(None, Some(vec![2.into()]), None, None).build();
      let filter = eth.new_logs_filter(filter).wait().unwrap();
      assert_eq!(filter.id, 0x123.into());
      filter.logs().wait()
    };

    // then
    assert_eq!(result, Ok(vec![log]));
    transport.assert_request("eth_newFilter", &[
      r#"{"address":null,"fromBlock":null,"limit":null,"toBlock":null,"topics":[null,["0x0000000000000000000000000000000000000000000000000000000000000002"],null,null]}"#.into()
    ]);
    transport.assert_request("eth_getFilterLogs", &[r#""0x123""#.into()]);
    transport.assert_request("eth_uninstallFilter", &[r#""0x123""#.into()]);
    transport.assert_no_more_requests();
  }

  #[test]
  fn logs_filter_poll() {
    // given
    let log = Log {
      address: 1.into(),
      topics: vec![],
      data: Bytes(vec![]),
      block_hash: Some(2.into()),
      block_number: Some(1.into()),
      transaction_hash: Some(3.into()),
      transaction_index: Some(0.into()),
      log_index: Some(0.into()),
      transaction_log_index: Some(0.into()),
      log_type: "mined".to_owned(),
    };

    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    transport.add_response(Value::Array(vec![
      serde_json::to_value(&log).unwrap(),
    ]));
    let result = {
      let eth = EthFilter::new(&transport);

      // when
      let filter = FilterBuilder::default().address(vec![2.into()]).build();
      let filter = eth.new_logs_filter(filter).wait().unwrap();
      assert_eq!(filter.id, 0x123.into());
      filter.poll().wait()
    };

    // then
    assert_eq!(result, Ok(Some(vec![log])));
    transport.assert_request("eth_newFilter", &[
      r#"{"address":["0x0000000000000000000000000000000000000002"],"fromBlock":null,"limit":null,"toBlock":null,"topics":null}"#.into()
    ]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_uninstallFilter", &[r#""0x123""#.into()]);
    transport.assert_no_more_requests();
  }

  #[test]
  fn blocks_filter() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    {
      let eth = EthFilter::new(&transport);

      // when
      let filter = eth.new_blocks_filter().wait().unwrap();
      assert_eq!(filter.id, 0x123.into());
    };

    // then
    transport.assert_request("eth_newBlockFilter", &[]);
    transport.assert_request("eth_uninstallFilter", &[r#""0x123""#.into()]);
    transport.assert_no_more_requests();
  }

  #[test]
  fn blocks_filter_poll() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into()),
    ]));
    let result = {
      let eth = EthFilter::new(&transport);

      // when
      let filter = eth.new_blocks_filter().wait().unwrap();
      assert_eq!(filter.id, 0x123.into());
      filter.poll().wait()
    };

    // then
    assert_eq!(result, Ok(Some(vec![0x456.into()])));
    transport.assert_request("eth_newBlockFilter", &[]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_uninstallFilter", &[r#""0x123""#.into()]);
    transport.assert_no_more_requests();
  }

  #[test]
  fn blocks_filter_stream() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into()),
    ]));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000457"#.into()),
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000458"#.into()),
    ]));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000459"#.into()),
    ]));
    let result = {
      let eth = EthFilter::new(&transport);

      // when
      let filter = eth.new_blocks_filter().wait().unwrap();
      filter.stream(Duration::from_secs(0)).take(4).collect().wait()
    };

    // then
    assert_eq!(result, Ok(vec![0x456.into(), 0x457.into(), 0x458.into(), 0x459.into()]));
    transport.assert_request("eth_newBlockFilter", &[]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_uninstallFilter", &[r#""0x123""#.into()]);
  }

  #[test]
  fn pending_transactions_filter() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    {
      let eth = EthFilter::new(&transport);

      // when
      let filter = eth.new_pending_transactions_filter().wait().unwrap();
      assert_eq!(filter.id, 0x123.into());
    };

    // then
    transport.assert_request("eth_newPendingTransactionFilter", &[]);
    transport.assert_request("eth_uninstallFilter", &[r#""0x123""#.into()]);
    transport.assert_no_more_requests();
  }

  #[test]
  fn new_pending_transactions_filter_poll() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into()),
    ]));
    let result = {
      let eth = EthFilter::new(&transport);

      // when
      let filter = eth.new_pending_transactions_filter().wait().unwrap();
      assert_eq!(filter.id, 0x123.into());
      filter.poll().wait()
    };

    // then
    assert_eq!(result, Ok(Some(vec![0x456.into()])));
    transport.assert_request("eth_newPendingTransactionFilter", &[]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_uninstallFilter", &[r#""0x123""#.into()]);
    transport.assert_no_more_requests();
  }
}

