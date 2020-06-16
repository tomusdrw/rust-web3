//! `Eth` namespace, filters.

use futures::{
    stream,
    task::{Context, Poll},
    Future, FutureExt, Stream, StreamExt,
};
use futures_timer::Delay;
use serde::de::DeserializeOwned;
use std::marker::{PhantomData, Unpin};
use std::pin::Pin;
use std::time::Duration;
use std::{fmt, vec};

use crate::api::Namespace;
use crate::helpers::{self, CallFuture};
use crate::types::{Filter, Log, H256};
use crate::{error, rpc, Transport};

fn interval(duration: Duration) -> impl Stream<Item = ()> + Send + Unpin {
    stream::unfold((), move |_| Delay::new(duration).map(|_| Some(((), ())))).map(drop)
}

/// Stream of events
pub struct FilterStream<T: Transport, I> {
    base: BaseFilter<T, I>,
    poll_interval: Duration,
    interval: Box<dyn Stream<Item = ()> + Send + Unpin>,
    state: FilterStreamState<I, T::Out>,
}

impl<T, I> fmt::Debug for FilterStream<T, I>
where
    T: Transport,
    T::Out: fmt::Debug,
    I: fmt::Debug + 'static,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("FilterStream")
            .field("base", &self.base)
            .field("interval", &self.poll_interval)
            .field("state", &self.state)
            .finish()
    }
}

impl<T: Transport, I> FilterStream<T, I> {
    fn new(base: BaseFilter<T, I>, poll_interval: Duration) -> Self {
        FilterStream {
            base,
            poll_interval: poll_interval.clone(),
            interval: Box::new(interval(poll_interval)),
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
    GetFilterChanges(CallFuture<Option<Vec<I>>, O>),
    NextItem(vec::IntoIter<I>),
}

impl<T: Transport, I: DeserializeOwned + Unpin> Stream for FilterStream<T, I> {
    type Item = error::Result<I>;

    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        loop {
            let next_state = match self.state {
                FilterStreamState::WaitForInterval => {
                    let _ready = ready!(self.interval.poll_next_unpin(ctx));
                    let id = helpers::serialize(&self.base.id);
                    let future = CallFuture::new(self.base.transport.execute("eth_getFilterChanges", vec![id]));
                    FilterStreamState::GetFilterChanges(future)
                }
                FilterStreamState::GetFilterChanges(ref mut future) => {
                    let items = ready!(future.poll_unpin(ctx))?.unwrap_or_default();
                    FilterStreamState::NextItem(items.into_iter())
                }
                FilterStreamState::NextItem(ref mut iter) => match iter.next() {
                    Some(item) => return Poll::Ready(Some(Ok(item))),
                    None => FilterStreamState::WaitForInterval,
                },
            };
            self.state = next_state;
        }
    }
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
    /// Polls this filter for changes.
    /// Will return logs that happened after previous poll.
    pub fn poll(&self) -> CallFuture<Option<Vec<I>>, T::Out> {
        let id = helpers::serialize(&self.id);
        CallFuture::new(self.transport.execute("eth_getFilterChanges", vec![id]))
    }

    /// Returns the stream of items which automatically polls the server
    pub fn stream(self, poll_interval: Duration) -> FilterStream<T, I> {
        FilterStream::new(self, poll_interval)
    }

    /// Uninstalls the filter
    pub fn uninstall(self) -> CallFuture<bool, T::Out>
    where
        Self: Sized,
    {
        self.uninstall_internal()
    }

    fn uninstall_internal(&self) -> CallFuture<bool, T::Out> {
        let id = helpers::serialize(&self.id);
        CallFuture::new(self.transport.execute("eth_uninstallFilter", vec![id]))
    }

    /// Borrows the transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> BaseFilter<T, Log> {
    /// Returns future with all logs matching given filter
    pub fn logs(&self) -> CallFuture<Vec<Log>, T::Out> {
        let id = helpers::serialize(&self.id);
        CallFuture::new(self.transport.execute("eth_getFilterLogs", vec![id]))
    }
}

/// Should be used to create new filter future
fn create_filter<T: Transport, F: FilterInterface>(t: T, arg: Vec<rpc::Value>) -> CreateFilter<T, F::Output> {
    let future = CallFuture::new(t.execute(F::constructor(), arg));
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
    future: CallFuture<String, T::Out>,
}

impl<T, I> Future for CreateFilter<T, I>
where
    T: Transport,
    I: Unpin,
{
    type Output = error::Result<BaseFilter<T, I>>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let id = ready!(self.future.poll_unpin(ctx))?;
        let result = BaseFilter {
            id,
            transport: self.transport.take().expect("future polled after ready; qed"),
            item: PhantomData,
        };
        Poll::Ready(Ok(result))
    }
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
    use crate::rpc::Value;
    use serde_json;
    use std::time::Duration;

    use crate::api::Namespace;
    use crate::helpers::tests::TestTransport;
    use crate::types::{Address, Bytes, FilterBuilder, Log, H256};

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
            futures::executor::block_on_stream(filter.stream(Duration::from_secs(0)))
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
