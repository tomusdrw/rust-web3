//! Easy to use utilities for confirmations.

use std::time::Duration;

use crate::api::{CreateFilter, Eth, EthFilter, FilterStream, Namespace};
use crate::helpers::CallFuture;
use crate::types::{Bytes, TransactionReceipt, TransactionRequest, H256, U64};
use crate::{Error, Transport};
use futures::stream::Skip;
use futures::{Future, IntoFuture, Poll, Stream};

/// Checks whether an event has been confirmed.
pub trait ConfirmationCheck {
    /// Future resolved when is known whether an event has been confirmed.
    type Check: IntoFuture<Item = Option<U64>, Error = Error>;

    /// Should be called to get future which resolves when confirmation state is known.
    fn check(&self) -> Self::Check;
}

impl<F, T> ConfirmationCheck for F
where
    F: Fn() -> T,
    T: IntoFuture<Item = Option<U64>, Error = Error>,
{
    type Check = T;

    fn check(&self) -> Self::Check {
        (*self)()
    }
}

enum WaitForConfirmationsState<F, O> {
    WaitForNextBlock,
    CheckConfirmation(F),
    CompareConfirmations(u64, CallFuture<U64, O>),
}

struct WaitForConfirmations<T, V, F>
where
    T: Transport,
{
    eth: Eth<T>,
    state: WaitForConfirmationsState<F, T::Out>,
    filter_stream: Skip<FilterStream<T, H256>>,
    confirmation_check: V,
    confirmations: usize,
}

impl<T, V, F> Future for WaitForConfirmations<T, V, F::Future>
where
    T: Transport,
    V: ConfirmationCheck<Check = F>,
    F: IntoFuture<Item = Option<U64>, Error = Error>,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                WaitForConfirmationsState::WaitForNextBlock => {
                    let _ = try_ready!(self.filter_stream.poll());
                    WaitForConfirmationsState::CheckConfirmation(self.confirmation_check.check().into_future())
                }
                WaitForConfirmationsState::CheckConfirmation(ref mut future) => match try_ready!(future.poll()) {
                    Some(confirmation_block_number) => {
                        let future = self.eth.block_number();
                        WaitForConfirmationsState::CompareConfirmations(confirmation_block_number.low_u64(), future)
                    }
                    None => WaitForConfirmationsState::WaitForNextBlock,
                },
                WaitForConfirmationsState::CompareConfirmations(
                    confirmation_block_number,
                    ref mut block_number_future,
                ) => {
                    let block_number = try_ready!(block_number_future.poll()).low_u64();
                    if confirmation_block_number + self.confirmations as u64 <= block_number {
                        return Ok(().into());
                    } else {
                        WaitForConfirmationsState::WaitForNextBlock
                    }
                }
            };
            self.state = next_state;
        }
    }
}

struct CreateWaitForConfirmations<T: Transport, V> {
    eth: Option<Eth<T>>,
    create_filter: CreateFilter<T, H256>,
    poll_interval: Duration,
    confirmation_check: Option<V>,
    confirmations: usize,
}

enum ConfirmationsState<T: Transport, V, F> {
    Create(CreateWaitForConfirmations<T, V>),
    Wait(WaitForConfirmations<T, V, F>),
}

/// On each new block checks confirmations.
pub struct Confirmations<T: Transport, V, F> {
    state: ConfirmationsState<T, V, F>,
}

impl<T: Transport, V, F> Confirmations<T, V, F> {
    fn new(eth: Eth<T>, eth_filter: EthFilter<T>, poll_interval: Duration, confirmations: usize, check: V) -> Self {
        Confirmations {
            state: ConfirmationsState::Create(CreateWaitForConfirmations {
                eth: Some(eth),
                create_filter: eth_filter.create_blocks_filter(),
                poll_interval,
                confirmation_check: Some(check),
                confirmations,
            }),
        }
    }
}

impl<T, V, F> Future for Confirmations<T, V, F::Future>
where
    T: Transport,
    V: ConfirmationCheck<Check = F>,
    F: IntoFuture<Item = Option<U64>, Error = Error>,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                ConfirmationsState::Create(ref mut create) => {
                    let filter = try_ready!(create.create_filter.poll());
                    let future = WaitForConfirmations {
                        eth: create.eth.take().expect("future polled after ready; qed"),
                        state: WaitForConfirmationsState::WaitForNextBlock,
                        filter_stream: filter.stream(create.poll_interval).skip(create.confirmations as u64),
                        confirmation_check: create
                            .confirmation_check
                            .take()
                            .expect("future polled after ready; qed"),
                        confirmations: create.confirmations,
                    };
                    ConfirmationsState::Wait(future)
                }
                ConfirmationsState::Wait(ref mut wait) => return Future::poll(wait),
            };
            self.state = next_state;
        }
    }
}

/// Should be used to wait for confirmations
pub fn wait_for_confirmations<T, V, F>(
    eth: Eth<T>,
    eth_filter: EthFilter<T>,
    poll_interval: Duration,
    confirmations: usize,
    check: V,
) -> Confirmations<T, V, F::Future>
where
    T: Transport,
    V: ConfirmationCheck<Check = F>,
    F: IntoFuture<Item = Option<U64>, Error = Error>,
{
    Confirmations::new(eth, eth_filter, poll_interval, confirmations, check)
}

struct TransactionReceiptBlockNumber<T: Transport> {
    future: CallFuture<Option<TransactionReceipt>, T::Out>,
}

impl<T: Transport> Future for TransactionReceiptBlockNumber<T> {
    type Item = Option<U64>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let receipt = try_ready!(self.future.poll());
        Ok(receipt.and_then(|receipt| receipt.block_number).into())
    }
}

struct TransactionReceiptBlockNumberCheck<T: Transport> {
    eth: Eth<T>,
    hash: H256,
}

impl<T: Transport> TransactionReceiptBlockNumberCheck<T> {
    fn new(eth: Eth<T>, hash: H256) -> Self {
        TransactionReceiptBlockNumberCheck { eth, hash }
    }
}

impl<T: Transport> ConfirmationCheck for TransactionReceiptBlockNumberCheck<T> {
    type Check = TransactionReceiptBlockNumber<T>;

    fn check(&self) -> Self::Check {
        TransactionReceiptBlockNumber {
            future: self.eth.transaction_receipt(self.hash),
        }
    }
}

enum SendTransactionWithConfirmationState<T: Transport> {
    Error(Option<Error>),
    SendTransaction(CallFuture<H256, T::Out>),
    WaitForConfirmations(
        H256,
        Confirmations<T, TransactionReceiptBlockNumberCheck<T>, TransactionReceiptBlockNumber<T>>,
    ),
    GetTransactionReceipt(CallFuture<Option<TransactionReceipt>, T::Out>),
}

/// Sends transaction and then checks if has been confirmed.
pub struct SendTransactionWithConfirmation<T: Transport> {
    state: SendTransactionWithConfirmationState<T>,
    transport: T,
    poll_interval: Duration,
    confirmations: usize,
}

impl<T: Transport> SendTransactionWithConfirmation<T> {
    fn new(transport: T, tx: TransactionRequest, poll_interval: Duration, confirmations: usize) -> Self {
        SendTransactionWithConfirmation {
            state: SendTransactionWithConfirmationState::SendTransaction(Eth::new(&transport).send_transaction(tx)),
            transport,
            poll_interval,
            confirmations,
        }
    }

    fn raw(transport: T, tx: Bytes, poll_interval: Duration, confirmations: usize) -> Self {
        SendTransactionWithConfirmation {
            state: SendTransactionWithConfirmationState::SendTransaction(Eth::new(&transport).send_raw_transaction(tx)),
            transport,
            poll_interval,
            confirmations,
        }
    }

    pub(crate) fn from_err<E: Into<Error>>(transport: T, err: E) -> Self {
        SendTransactionWithConfirmation {
            state: SendTransactionWithConfirmationState::Error(Some(err.into())),
            transport,
            poll_interval: Duration::from_secs(1),
            confirmations: 1,
        }
    }
}

impl<T: Transport> Future for SendTransactionWithConfirmation<T> {
    type Item = TransactionReceipt;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            let next_state = match self.state {
                SendTransactionWithConfirmationState::Error(ref mut error) => {
                    return Err(error
                        .take()
                        .expect("Error is initialized initially; future polled only once; qed"));
                }
                SendTransactionWithConfirmationState::SendTransaction(ref mut future) => {
                    let hash = try_ready!(future.poll());
                    if self.confirmations > 0 {
                        let confirmation_check =
                            TransactionReceiptBlockNumberCheck::new(Eth::new(self.transport.clone()), hash);
                        let eth = Eth::new(self.transport.clone());
                        let eth_filter = EthFilter::new(self.transport.clone());
                        let wait = wait_for_confirmations(
                            eth,
                            eth_filter,
                            self.poll_interval,
                            self.confirmations,
                            confirmation_check,
                        );
                        SendTransactionWithConfirmationState::WaitForConfirmations(hash, wait)
                    } else {
                        let receipt_future = Eth::new(&self.transport).transaction_receipt(hash);
                        SendTransactionWithConfirmationState::GetTransactionReceipt(receipt_future)
                    }
                }
                SendTransactionWithConfirmationState::WaitForConfirmations(hash, ref mut future) => {
                    let _confirmed = try_ready!(Future::poll(future));
                    let receipt_future = Eth::new(&self.transport).transaction_receipt(hash);
                    SendTransactionWithConfirmationState::GetTransactionReceipt(receipt_future)
                }
                SendTransactionWithConfirmationState::GetTransactionReceipt(ref mut future) => {
                    let receipt = try_ready!(Future::poll(future))
                        .expect("receipt can't be null after wait for confirmations; qed");
                    return Ok(receipt.into());
                }
            };
            self.state = next_state;
        }
    }
}

/// Sends transaction and returns future resolved after transaction is confirmed
pub fn send_transaction_with_confirmation<T>(
    transport: T,
    tx: TransactionRequest,
    poll_interval: Duration,
    confirmations: usize,
) -> SendTransactionWithConfirmation<T>
where
    T: Transport,
{
    SendTransactionWithConfirmation::new(transport, tx, poll_interval, confirmations)
}

/// Sends raw transaction and returns future resolved after transaction is confirmed
pub fn send_raw_transaction_with_confirmation<T>(
    transport: T,
    tx: Bytes,
    poll_interval: Duration,
    confirmations: usize,
) -> SendTransactionWithConfirmation<T>
where
    T: Transport,
{
    SendTransactionWithConfirmation::raw(transport, tx, poll_interval, confirmations)
}

#[cfg(test)]
mod tests {
    use super::send_transaction_with_confirmation;
    use crate::helpers::tests::TestTransport;
    use crate::rpc::Value;
    use crate::types::{Address, TransactionReceipt, TransactionRequest, H256, U64};
    use futures::Future;
    use serde_json::json;
    use std::time::Duration;

    #[test]
    fn test_send_transaction_with_confirmation() {
        let mut transport = TestTransport::default();
        let confirmations = 3;
        let transaction_request = TransactionRequest {
            from: Address::from_low_u64_be(0x123),
            to: Some(Address::from_low_u64_be(0x123)),
            gas: None,
            gas_price: Some(1.into()),
            value: Some(1.into()),
            data: None,
            nonce: None,
            condition: None,
        };

        let transaction_receipt = TransactionReceipt {
            transaction_hash: H256::zero(),
            transaction_index: U64::zero(),
            block_hash: Some(H256::zero()),
            block_number: Some(2.into()),
            cumulative_gas_used: 0.into(),
            gas_used: Some(0.into()),
            contract_address: None,
            logs: vec![],
            status: Some(1.into()),
            root: Some(H256::zero()),
            logs_bloom: Default::default(),
        };

        let poll_interval = Duration::from_secs(0);
        transport.add_response(Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000111"#.into(),
        ));
        transport.add_response(Value::String("0x123".into()));
        transport.add_response(Value::Array(vec![
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000456"#.into()),
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000457"#.into()),
        ]));
        transport.add_response(Value::Array(vec![Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000458"#.into(),
        )]));
        transport.add_response(Value::Array(vec![Value::String(
            r#"0x0000000000000000000000000000000000000000000000000000000000000459"#.into(),
        )]));
        transport.add_response(Value::Null);
        transport.add_response(Value::Array(vec![
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000460"#.into()),
            Value::String(r#"0x0000000000000000000000000000000000000000000000000000000000000461"#.into()),
        ]));
        transport.add_response(Value::Null);
        transport.add_response(json!(transaction_receipt));
        transport.add_response(Value::String("0x6".into()));
        transport.add_response(json!(transaction_receipt));
        transport.add_response(Value::Bool(true));

        let confirmation = {
            let future =
                send_transaction_with_confirmation(&transport, transaction_request, poll_interval, confirmations);
            future.wait()
        };

        transport.assert_request("eth_sendTransaction", &[r#"{"from":"0x0000000000000000000000000000000000000123","gasPrice":"0x1","to":"0x0000000000000000000000000000000000000123","value":"0x1"}"#.into()]);
        transport.assert_request("eth_newBlockFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()],
        );
        transport.assert_request("eth_getFilterChanges", &[r#""0x123""#.into()]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()],
        );
        transport.assert_request(
            "eth_getTransactionReceipt",
            &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()],
        );
        transport.assert_request("eth_blockNumber", &[]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &[r#""0x0000000000000000000000000000000000000000000000000000000000000111""#.into()],
        );
        transport.assert_no_more_requests();
        assert_eq!(confirmation, Ok(transaction_receipt));
    }
}
