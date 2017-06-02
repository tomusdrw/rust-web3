//! `Eth` namespace, filters.

use std::marker::PhantomData;
use std::ops::Deref;

use api::Namespace;
use futures::{Async, Poll, Future};
use helpers::{self, CallResult};
use types::{Filter, H256, Log, U256};
use {Transport, Error};

/// `Eth` namespace, filters
pub struct EthFilter<T> {
  transport: T,
}

impl<T: Transport + Clone> Namespace<T> for EthFilter<T> {
  fn new(transport: T) -> Self where Self: Sized {
    EthFilter {
      transport: transport,
    }
  }
}

impl<T: Transport + Clone> EthFilter<T> {
  /// Installs a new logs filter.
  pub fn new_logs_filter(&self, filter: Filter) -> FilterResult<T, CallResult<U256, T::Out>, LogsFilter<T>> {
    let f = helpers::serialize(&filter);
    FilterResult::new(
      self.transport.clone(),
      CallResult::new(self.transport.execute("eth_newFilter", vec![f]))
    )
  }

  /// Installs a new block filter.
  pub fn new_blocks_filter(&self) -> FilterResult<T, CallResult<U256, T::Out>, BlocksFilter<T>> {
    FilterResult::new(
      self.transport.clone(),
      CallResult::new(self.transport.execute("eth_newBlockFilter", vec![]))
    )
  }

  /// Installs a new pending transactions filter.
  pub fn new_pending_transactions_filter(&self) -> FilterResult<T, CallResult<U256, T::Out>, PendingTransactionsFilter<T>> {
    FilterResult::new(
      self.transport.clone(),
      CallResult::new(self.transport.execute("eth_newPendingTransactionFilter", vec![]))
    )
  }
}

/// Result of installing new filter
pub struct FilterResult<T, F, X> {
  transport: T,
  inner: F,
  _output: PhantomData<X>,
}

impl<T, F, X> FilterResult<T, F, X> {
  /// Create a new `FilterResult` wrapping the inner future.
  pub fn new(transport: T, inner: F) -> Self {
    FilterResult { transport: transport, inner: inner, _output: PhantomData }
  }
}

impl<T, F, X> Future for FilterResult<T, F, X> where
  T: Transport + Clone,
  F: Future<Item=U256, Error=Error>,
  X: FilterHandle<T>,
{
  type Item = X;
  type Error = Error;

  fn poll(&mut self) -> Poll<X, Error> {
    match self.inner.poll() {
      Ok(Async::Ready(x)) => Ok(Async::Ready(X::new(x, self.transport.clone()))),
      Ok(Async::NotReady) => Ok(Async::NotReady),
      Err(e) => Err(e),
    }
  }
}

/// Generic filter interface
pub trait FilterHandle<T: Transport> {
  /// Creates a new filter.
  fn new(id: U256, transport: T) -> Self;
}

/// Base filter handle.
/// Uninstall filter on drop.
/// Allows to poll the filter.
pub struct BaseFilter<T: Transport, R> {
  id: U256,
  transport: T,
  _output: PhantomData<R>,
}

impl<T: Transport, R> BaseFilter<T, R> {
  /// Polls this filter for changes.
  /// Will return logs that happened after previous poll.
  pub fn poll(&self) -> CallResult<Option<Vec<R>>, T::Out> {
    let id = helpers::serialize(&self.id);
    CallResult::new(self.transport.execute("eth_getFilterChanges", vec![id]))
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

impl<T: Transport, R> Drop for BaseFilter<T, R> {
  fn drop(&mut self) {
    let _ = self.uninstall_internal().wait();
  }
}

/// Logs Filter
pub struct LogsFilter<T: Transport> {
  base: BaseFilter<T, Log>,
}

impl<T: Transport> FilterHandle<T> for LogsFilter<T> {
  fn new(id: U256, transport: T) -> Self {
    LogsFilter {
      base: BaseFilter {
        id: id,
        transport: transport,
        _output: PhantomData,
      }
    }
  }
}

impl<T: Transport> LogsFilter<T> {
  /// Get all logs matching the filter.
  pub fn logs(&self) -> CallResult<Vec<Log>, T::Out> {
    let id = helpers::serialize(&self.id);
    CallResult::new(self.transport.execute("eth_getFilterLogs", vec![id]))
  }
}

impl<T: Transport> Deref for LogsFilter<T> {
  type Target = BaseFilter<T, Log>;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

/// New blocks hashes filter.
pub struct BlocksFilter<T: Transport> {
  base: BaseFilter<T, H256>,
}

impl<T: Transport> FilterHandle<T> for BlocksFilter<T> {
  fn new(id: U256, transport: T) -> Self {
    BlocksFilter {
      base: BaseFilter {
        id: id,
        transport: transport,
        _output: PhantomData,
      }
    }
  }
}

impl<T: Transport> Deref for BlocksFilter<T> {
  type Target = BaseFilter<T, H256>;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

/// New Pending Transactions Filter
pub struct PendingTransactionsFilter<T: Transport> {
  base: BaseFilter<T, H256>,
}

impl<T: Transport> FilterHandle<T> for PendingTransactionsFilter<T> {
  fn new(id: U256, transport: T) -> Self {
    PendingTransactionsFilter {
      base: BaseFilter {
        id: id,
        transport: transport,
        _output: PhantomData,
      }
    }
  }
}

impl<T: Transport> Deref for PendingTransactionsFilter<T> {
  type Target = BaseFilter<T, H256>;

  fn deref(&self) -> &Self::Target {
    &self.base
  }
}

#[cfg(test)]
mod tests {
  use futures::Future;

  use api::Namespace;
  use helpers::tests::TestTransport;
  use types::{Bytes, Log, FilterBuilder};
  use rpc::Value;
  use serde_json;

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
