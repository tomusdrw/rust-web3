//! `Eth` namespace, filters.


use std::marker::PhantomData;
use std::time::Duration;
use std::vec;
use serde::de::DeserializeOwned;
use tokio_timer::{Timer, Interval};
use futures::{Poll, Future, Stream};

use api::Namespace;
use helpers::{self, CallResult};
use types::{Filter, H256, Log};
use {Transport, Error, ErrorKind, rpc};

/// Stream of events
#[derive(Debug)]
pub struct FilterStream<T: Transport, I> {
  base: BaseFilter<T, I>,
  interval: Interval,
  state: FilterStreamState<I, T::Out>,
}

impl<T: Transport, I> FilterStream<T, I> {
  fn new(base: BaseFilter<T, I>, poll_interval: Duration) -> Self {
    FilterStream {
      base,
      interval: Timer::default().interval(poll_interval),
      state: FilterStreamState::WaitForInterval,
    }
  }

  /// Borrow a transport from this filter.
  pub fn transport(&self) -> &T {
    self.base.transport()
  }
}

#[derive(Debug)]
enum FilterStreamState<I, O> {
  WaitForInterval,
  GetFilterChanges(CallResult<Option<Vec<I>>, O>),
  NextItem(vec::IntoIter<I>),
}

impl<T: Transport, I: DeserializeOwned> Stream for FilterStream<T, I> {
  type Item = I;
  type Error = Error;

  fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
    loop {
      let next_state = match self.state {
        FilterStreamState::WaitForInterval => {
          let _ready = try_ready!(self.interval.poll().map_err(|_| Error::from(ErrorKind::Unreachable)));
          let id = helpers::serialize(&self.base.id);
          let future = CallResult::new(self.base.transport.execute("eth_getFilterChanges", vec![id]));
          FilterStreamState::GetFilterChanges(future)
        },
        FilterStreamState::GetFilterChanges(ref mut future) => {
          let items = try_ready!(future.poll()).unwrap_or_default();
          FilterStreamState::NextItem(items.into_iter())
        },
        FilterStreamState::NextItem(ref mut iter) => match iter.next() {
          Some(item) => return Ok(Some(item).into()),
          None => FilterStreamState::WaitForInterval
        }
      };
      self.state = next_state;
    }
  }
}

/// Specifies filter items and constructor method.
trait FilterInterface {
  /// Filter item type
  type Item;

  /// Name of method used to construct the filter
  fn constructor() -> &'static str;
}

/// Logs Filter
#[derive(Debug)]
struct LogsFilter;

impl FilterInterface for LogsFilter {
  type Item = Log;

  fn constructor() -> &'static str {
    "eth_newFilter"
  }
}

/// New blocks hashes filter.
#[derive(Debug)]
struct BlocksFilter;

impl FilterInterface for BlocksFilter {
  type Item = H256;

  fn constructor() -> &'static str {
    "eth_newBlockFilter"
  }
}

/// New Pending Transactions Filter
#[derive(Debug)]
struct PendingTransactionsFilter;

impl FilterInterface for PendingTransactionsFilter {
  type Item = H256;

  fn constructor() -> &'static str {
    "eth_newPendingTransactionFilter"
  }
}

/// Base filter handle.
/// Uninstall filter on drop.
/// Allows to poll the filter.
#[derive(Debug)]
pub struct BaseFilter<T: Transport, I> {
  // TODO [ToDr] Workaround for ganache returning 0x03 instead of 0x3
  id: String,
  transport: T,
  item: PhantomData<I>,
}

impl<T: Transport, I> BaseFilter<T, I> {
  /// Polls this filter for changes.
  /// Will return logs that happened after previous poll.
  pub fn poll(&self) -> CallResult<Option<Vec<I>>, T::Out> {
    let id = helpers::serialize(&self.id);
    CallResult::new(self.transport.execute("eth_getFilterChanges", vec![id]))
  }

  /// Returns the stream of items which automatically polls the server
  pub fn stream(self, poll_interval: Duration) -> FilterStream<T, I> {
    FilterStream::new(self, poll_interval)
  }

  /// Uninstalls the filter
  pub fn uninstall(self) -> CallResult<bool, T::Out> where Self: Sized {
    self.uninstall_internal()
  }

  fn uninstall_internal(&self) -> CallResult<bool, T::Out> {
    let id = helpers::serialize(&self.id);
    CallResult::new(self.transport.execute("eth_uninstallFilter", vec![id]))
  }

  /// Borrows the transport.
  pub fn transport(&self) -> &T {
    &self.transport
  }
}

impl<T: Transport> BaseFilter<T, Log> {
  /// Returns future with all logs matching given filter
  pub fn logs(&self) -> CallResult<Vec<Log>, T::Out> {
    let id = helpers::serialize(&self.id);
    CallResult::new(self.transport.execute("eth_getFilterLogs", vec![id]))
  }
}

/// Should be used to create new filter future
fn create_filter<T: Transport, F: FilterInterface>(t: T, arg: Vec<rpc::Value>) -> CreateFilter<T, F::Item> {
  let future = CallResult::new(t.execute(F::constructor(), arg));
  CreateFilter {
    transport: Some(t),
    item: PhantomData,
    future,
  }
}

/// Future which resolves with new filter
#[derive(Debug)]
pub struct CreateFilter<T: Transport, I> {
  transport: Option<T>,
  item: PhantomData<I>,
  future: CallResult<String, T::Out>,
}

impl<T, I> Future for CreateFilter<T, I> where
  T: Transport,
{
  type Item = BaseFilter<T, I>;
  type Error = Error;

  fn poll(&mut self) -> Poll<Self::Item, Error> {
    let id = try_ready!(self.future.poll());
    let result = BaseFilter {
      id,
      transport: self.transport.take().expect("future polled after ready; qed"),
      item: PhantomData,
    };
    Ok(result.into())
  }
}

/// `Eth` namespace, filters
#[derive(Debug, Clone)]
pub struct EthFilter<T> {
  transport: T,
}

impl<T: Transport> Namespace<T> for EthFilter<T> {
  fn new(transport: T) -> Self where Self: Sized {
    EthFilter {
      transport,
    }
  }

  fn transport(&self) -> &T {
    &self.transport
  }
}

impl<T: Transport> EthFilter<T> {
  /// Installs a new logs filter.
  pub fn create_logs_filter(self, filter: Filter) -> CreateFilter<T, Log> {
    let f = helpers::serialize(&filter);
    create_filter::<_, LogsFilter>(self.transport, vec![f])
  }

  /// Installs a new block filter.
  pub fn create_blocks_filter(self) -> CreateFilter<T, H256> {
    create_filter::<_, BlocksFilter>(self.transport, vec![])
  }

  /// Installs a new pending transactions filter.
  pub fn create_pending_transactions_filter(self) -> CreateFilter<T, H256> {
    create_filter::<_, PendingTransactionsFilter>(self.transport, vec![])
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
      let filter = eth.create_logs_filter(filter).wait().unwrap();
      assert_eq!(filter.id, "0x123".to_owned());
    };

    // then
    transport.assert_request("eth_newFilter", &[
      r#"{"address":null,"fromBlock":null,"limit":10,"toBlock":null,"topics":null}"#.into()
    ]);
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
      log_type: Some("mined".into()),
      removed: None,
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
      let filter = eth.create_logs_filter(filter).wait().unwrap();
      assert_eq!(filter.id, "0x123".to_owned());
      filter.logs().wait()
    };

    // then
    assert_eq!(result, Ok(vec![log]));
    transport.assert_request("eth_newFilter", &[
      r#"{"address":null,"fromBlock":null,"limit":null,"toBlock":null,"topics":[null,["0x0000000000000000000000000000000000000000000000000000000000000002"],null,null]}"#.into()
    ]);
    transport.assert_request("eth_getFilterLogs", &[r#""0x123""#.into()]);
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
      log_type: Some("mined".into()),
      removed: None,
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
      let filter = eth.create_logs_filter(filter).wait().unwrap();
      assert_eq!(filter.id, "0x123".to_owned());
      filter.poll().wait()
    };

    // then
    assert_eq!(result, Ok(Some(vec![log])));
    transport.assert_request("eth_newFilter", &[
      r#"{"address":["0x0000000000000000000000000000000000000002"],"fromBlock":null,"limit":null,"toBlock":null,"topics":null}"#.into()
    ]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
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
      let filter = eth.create_blocks_filter().wait().unwrap();
      assert_eq!(filter.id, "0x123".to_owned());
    };

    // then
    transport.assert_request("eth_newBlockFilter", &[]);
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
      let filter = eth.create_blocks_filter().wait().unwrap();
      assert_eq!(filter.id, "0x123".to_owned());
      filter.poll().wait()
    };

    // then
    assert_eq!(result, Ok(Some(vec![0x456.into()])));
    transport.assert_request("eth_newBlockFilter", &[]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
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
      let filter = eth.create_blocks_filter().wait().unwrap();
      filter.stream(Duration::from_secs(0)).take(4).collect().wait()
    };

    // then
    assert_eq!(result, Ok(vec![0x456.into(), 0x457.into(), 0x458.into(), 0x459.into()]));
    transport.assert_request("eth_newBlockFilter", &[]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
  }

  #[test]
  fn pending_transactions_filter() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    {
      let eth = EthFilter::new(&transport);

      // when
      let filter = eth.create_pending_transactions_filter().wait().unwrap();
      assert_eq!(filter.id, "0x123".to_owned());
    };

    // then
    transport.assert_request("eth_newPendingTransactionFilter", &[]);
    transport.assert_no_more_requests();
  }

  #[test]
  fn create_pending_transactions_filter_poll() {
    // given
    let mut transport = TestTransport::default();
    transport.set_response(Value::String("0x123".into()));
    transport.add_response(Value::Array(vec![
      Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into()),
    ]));
    let result = {
      let eth = EthFilter::new(&transport);

      // when
      let filter = eth.create_pending_transactions_filter().wait().unwrap();
      assert_eq!(filter.id, "0x123".to_owned());
      filter.poll().wait()
    };

    // then
    assert_eq!(result, Ok(Some(vec![0x456.into()])));
    transport.assert_request("eth_newPendingTransactionFilter", &[]);
    transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
    transport.assert_no_more_requests();
  }
}

