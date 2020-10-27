//! `Eth` namespace, filters.

use futures::{stream, Stream, TryStreamExt};
use futures_timer::Delay;
use serde::de::DeserializeOwned;
use std::marker::PhantomData;
use std::time::Duration;
use std::{fmt, vec};

use crate::api::Namespace;
use crate::helpers;
use crate::types::{Filter, Log, H256};
use crate::{error, rpc, Transport};

fn filter_stream<T: Transport, I: DeserializeOwned>(
    base: BaseFilter<T, I>,
    poll_interval: Duration,
) -> impl Stream<Item = error::Result<I>> {
    let id = helpers::serialize(&base.id);
    stream::unfold((base, id), move |state| async move {
        let (base, id) = state;
        Delay::new(poll_interval).await;
        let response = base.transport.execute("eth_getFilterChanges", vec![id.clone()]).await;
        let items: error::Result<Option<Vec<I>>> = response.and_then(helpers::decode);
        let items = items.map(Option::unwrap_or_default);
        Some((items, (base, id)))
    })
    // map I to Result<I> even though it is always Ok so that try_flatten works
    .map_ok(|items| stream::iter(items.into_iter().map(Ok)))
    .try_flatten()
    .into_stream()
}

/// Specifies filter items and constructor method.
trait FilterInterface {
    /// Filter item type
    type Output;

    /// Name of method used to construct the filter
    fn constructor() -> &'static str;
}

/// Logs Filter
#[derive(Debug)]
struct LogsFilter;

impl FilterInterface for LogsFilter {
    type Output = Log;

    fn constructor() -> &'static str {
        "eth_newFilter"
    }
}

/// New blocks hashes filter.
#[derive(Debug)]
struct BlocksFilter;

impl FilterInterface for BlocksFilter {
    type Output = H256;

    fn constructor() -> &'static str {
        "eth_newBlockFilter"
    }
}

/// New Pending Transactions Filter
#[derive(Debug)]
struct PendingTransactionsFilter;

impl FilterInterface for PendingTransactionsFilter {
    type Output = H256;

    fn constructor() -> &'static str {
        "eth_newPendingTransactionFilter"
    }
}

/// Base filter handle.
/// Uninstall filter on drop.
/// Allows to poll the filter.
pub struct BaseFilter<T: Transport, I> {
    // TODO [ToDr] Workaround for ganache returning 0x03 instead of 0x3
    id: String,
    transport: T,
    item: PhantomData<I>,
}

impl<T: Transport, I: 'static> fmt::Debug for BaseFilter<T, I> {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("BaseFilter")
            .field("id", &self.id)
            .field("transport", &self.transport)
            .field("item", &std::any::TypeId::of::<I>())
            .finish()
    }
}

impl<T: Transport, I> Clone for BaseFilter<T, I> {
    fn clone(&self) -> Self {
        BaseFilter {
            id: self.id.clone(),
            transport: self.transport.clone(),
            item: PhantomData::default(),
        }
    }
}

impl<T: Transport, I> BaseFilter<T, I> {
    /// Uninstalls the filter
    pub async fn uninstall(self) -> error::Result<bool>
    where
        Self: Sized,
    {
        let id = helpers::serialize(&self.id);
        let response = self.transport.execute("eth_uninstallFilter", vec![id]).await?;
        helpers::decode(response)
    }

    /// Borrows the transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport, I: DeserializeOwned> BaseFilter<T, I> {
    /// Polls this filter for changes.
    /// Will return logs that happened after previous poll.
    pub async fn poll(&self) -> error::Result<Option<Vec<I>>> {
        let id = helpers::serialize(&self.id);
        let response = self.transport.execute("eth_getFilterChanges", vec![id]).await?;
        helpers::decode(response)
    }

    /// Returns the stream of items which automatically polls the server
    pub fn stream(self, poll_interval: Duration) -> impl Stream<Item = error::Result<I>> {
        filter_stream(self, poll_interval)
    }
}

impl<T: Transport> BaseFilter<T, Log> {
    /// Returns future with all logs matching given filter
    pub async fn logs(&self) -> error::Result<Vec<Log>> {
        let id = helpers::serialize(&self.id);
        let response = self.transport.execute("eth_getFilterLogs", vec![id]).await?;
        helpers::decode(response)
    }
}

/// Should be used to create new filter future
async fn create_filter<T: Transport, F: FilterInterface>(
    transport: T,
    arg: Vec<rpc::Value>,
) -> error::Result<BaseFilter<T, F::Output>> {
    let response = transport.execute(F::constructor(), arg).await?;
    let id = helpers::decode(response)?;
    Ok(BaseFilter {
        id,
        transport,
        item: PhantomData,
    })
}

/// `Eth` namespace, filters
#[derive(Debug, Clone)]
pub struct EthFilter<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for EthFilter<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        EthFilter { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> EthFilter<T> {
    /// Installs a new logs filter.
    pub async fn create_logs_filter(self, filter: Filter) -> error::Result<BaseFilter<T, Log>> {
        let f = helpers::serialize(&filter);
        create_filter::<_, LogsFilter>(self.transport, vec![f]).await
    }

    /// Installs a new block filter.
    pub async fn create_blocks_filter(self) -> error::Result<BaseFilter<T, H256>> {
        create_filter::<_, BlocksFilter>(self.transport, vec![]).await
    }

    /// Installs a new pending transactions filter.
    pub async fn create_pending_transactions_filter(self) -> error::Result<BaseFilter<T, H256>> {
        create_filter::<_, PendingTransactionsFilter>(self.transport, vec![]).await
    }
}

#[cfg(test)]
mod tests {
    use super::EthFilter;
    use crate::api::Namespace;
    use crate::helpers::tests::TestTransport;
    use crate::rpc::Value;
    use crate::types::{Address, Bytes, FilterBuilder, Log, H256};
    use futures::stream::StreamExt;
    use std::time::Duration;

    #[test]
    fn logs_filter() {
        // given
        let mut transport = TestTransport::default();
        transport.set_response(Value::String("0x123".into()));
        {
            let eth = EthFilter::new(&transport);

            // when
            let filter = FilterBuilder::default().limit(10).build();
            let filter = futures::executor::block_on(eth.create_logs_filter(filter)).unwrap();
            assert_eq!(filter.id, "0x123".to_owned());
        };

        // then
        transport.assert_request("eth_newFilter", &[r#"{"limit":10}"#.into()]);
        transport.assert_no_more_requests();
    }

    #[test]
    fn logs_filter_get_logs() {
        // given
        let log = Log {
            address: Address::from_low_u64_be(1),
            topics: vec![],
            data: Bytes(vec![]),
            block_hash: Some(H256::from_low_u64_be(2)),
            block_number: Some(1.into()),
            transaction_hash: Some(H256::from_low_u64_be(3)),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: Some("mined".into()),
            removed: None,
        };

        let mut transport = TestTransport::default();
        transport.set_response(Value::String("0x123".into()));
        transport.add_response(Value::Array(vec![serde_json::to_value(&log).unwrap()]));
        let result = {
            let eth = EthFilter::new(&transport);

            // when
            let filter = FilterBuilder::default()
                .topics(None, Some(vec![H256::from_low_u64_be(2)]), None, None)
                .build();
            let filter = futures::executor::block_on(eth.create_logs_filter(filter)).unwrap();
            assert_eq!(filter.id, "0x123".to_owned());
            futures::executor::block_on(filter.logs())
        };

        // then
        assert_eq!(result, Ok(vec![log]));
        transport.assert_request(
            "eth_newFilter",
            &[r#"{"topics":[null,"0x0000000000000000000000000000000000000000000000000000000000000002"]}"#.into()],
        );
        transport.assert_request("eth_getFilterLogs", &[r#""0x123""#.into()]);
        transport.assert_no_more_requests();
    }

    #[test]
    fn logs_filter_poll() {
        // given
        let log = Log {
            address: Address::from_low_u64_be(1),
            topics: vec![],
            data: Bytes(vec![]),
            block_hash: Some(H256::from_low_u64_be(2)),
            block_number: Some(1.into()),
            transaction_hash: Some(H256::from_low_u64_be(3)),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: Some("mined".into()),
            removed: None,
        };

        let mut transport = TestTransport::default();
        transport.set_response(Value::String("0x123".into()));
        transport.add_response(Value::Array(vec![serde_json::to_value(&log).unwrap()]));
        let result = {
            let eth = EthFilter::new(&transport);

            // when
            let filter = FilterBuilder::default()
                .address(vec![Address::from_low_u64_be(2)])
                .build();
            let filter = futures::executor::block_on(eth.create_logs_filter(filter)).unwrap();
            assert_eq!(filter.id, "0x123".to_owned());
            futures::executor::block_on(filter.poll())
        };

        // then
        assert_eq!(result, Ok(Some(vec![log])));
        transport.assert_request(
            "eth_newFilter",
            &[r#"{"address":"0x0000000000000000000000000000000000000002"}"#.into()],
        );
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
            let filter = futures::executor::block_on(eth.create_blocks_filter()).unwrap();
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
        transport.add_response(Value::Array(vec![Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into(),
        )]));
        let result = {
            let eth = EthFilter::new(&transport);

            // when
            let filter = futures::executor::block_on(eth.create_blocks_filter()).unwrap();
            assert_eq!(filter.id, "0x123".to_owned());
            futures::executor::block_on(filter.poll())
        };

        // then
        assert_eq!(result, Ok(Some(vec![H256::from_low_u64_be(0x456)])));
        transport.assert_request("eth_newBlockFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_no_more_requests();
    }

    #[test]
    fn blocks_filter_stream() {
        // given
        let mut transport = TestTransport::default();
        transport.set_response(Value::String("0x123".into()));
        transport.add_response(Value::Array(vec![Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into(),
        )]));
        transport.add_response(Value::Array(vec![
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000457"#.into()),
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000458"#.into()),
        ]));
        transport.add_response(Value::Array(vec![Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000459"#.into(),
        )]));
        let result: Vec<_> = {
            let eth = EthFilter::new(&transport);

            // when
            let filter = futures::executor::block_on(eth.create_blocks_filter()).unwrap();
            futures::executor::block_on_stream(filter.stream(Duration::from_secs(0)).boxed_local())
                .take(4)
                .collect()
        };

        // then
        assert_eq!(
            result,
            [0x456, 0x457, 0x458, 0x459]
                .iter()
                .copied()
                .map(H256::from_low_u64_be)
                .map(Ok)
                .collect::<Vec<_>>()
        );
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
            let filter = futures::executor::block_on(eth.create_pending_transactions_filter()).unwrap();
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
        transport.add_response(Value::Array(vec![Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into(),
        )]));
        let result = {
            let eth = EthFilter::new(&transport);

            // when
            let filter = futures::executor::block_on(eth.create_pending_transactions_filter()).unwrap();
            assert_eq!(filter.id, "0x123".to_owned());
            futures::executor::block_on(filter.poll())
        };

        // then
        assert_eq!(result, Ok(Some(vec![H256::from_low_u64_be(0x456)])));
        transport.assert_request("eth_newPendingTransactionFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_no_more_requests();
    }
}
