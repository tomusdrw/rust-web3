//! `Web3` implementation

mod eth;
mod eth_filter;
mod eth_subscribe;
mod net;
mod personal;
mod web3;

pub use self::eth::Eth;
pub use self::eth_filter::{BaseFilter, CreateFilter, EthFilter, FilterStream};
pub use self::eth_subscribe::{SubscriptionId, SubscriptionStream};
pub use self::net::Net;
pub use self::personal::Personal;
pub use self::web3::Web3 as Web3Api;

use std::time::Duration;
use futures::IntoFuture;
use {confirm, DuplexTransport, Error, Transport};
use types::{Bytes, TransactionRequest, U256};

/// Common API for all namespaces
pub trait Namespace<T: Transport>: Clone {
    /// Creates new API namespace
    fn new(transport: T) -> Self;

    /// Borrows a transport.
    fn transport(&self) -> &T;
}

/// `Web3` wrapper for all namespaces
#[derive(Debug, Clone)]
pub struct Web3<T: Transport> {
    transport: T,
}

impl<T: Transport> Web3<T> {
    /// Create new `Web3` with given transport
    pub fn new(transport: T) -> Self {
        Web3 { transport }
    }

    /// Borrows a transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Access methods from custom namespace
    pub fn api<A: Namespace<T>>(&self) -> A {
        A::new(self.transport.clone())
    }

    /// Access methods from `eth` namespace
    pub fn eth(&self) -> eth::Eth<T> {
        self.api()
    }

    /// Access methods from `net` namespace
    pub fn net(&self) -> net::Net<T> {
        self.api()
    }

    /// Access methods from `web3` namespace
    pub fn web3(&self) -> web3::Web3<T> {
        self.api()
    }

    /// Access filter methods from `eth` namespace
    pub fn eth_filter(&self) -> eth_filter::EthFilter<T> {
        self.api()
    }

    /// Access methods from `personal` namespace
    pub fn personal(&self) -> personal::Personal<T> {
        self.api()
    }

    /// Should be used to wait for confirmations
    pub fn wait_for_confirmations<F, V>(&self, poll_interval: Duration, confirmations: usize, check: V) -> confirm::Confirmations<T, V, F::Future>
    where
        F: IntoFuture<Item = Option<U256>, Error = Error>,
        V: confirm::ConfirmationCheck<Check = F>,
    {
        confirm::wait_for_confirmations(
            self.eth(),
            self.eth_filter(),
            poll_interval,
            confirmations,
            check,
        )
    }

    /// Sends transaction and returns future resolved after transaction is confirmed
    pub fn send_transaction_with_confirmation(&self, tx: TransactionRequest, poll_interval: Duration, confirmations: usize) -> confirm::SendTransactionWithConfirmation<T> {
        confirm::send_transaction_with_confirmation(self.transport.clone(), tx, poll_interval, confirmations)
    }

    /// Sends raw transaction and returns future resolved after transaction is confirmed
    pub fn send_raw_transaction_with_confirmation(&self, tx: Bytes, poll_interval: Duration, confirmations: usize) -> confirm::SendTransactionWithConfirmation<T> {
        confirm::send_raw_transaction_with_confirmation(self.transport.clone(), tx, poll_interval, confirmations)
    }
}

impl<T: DuplexTransport> Web3<T> {
    /// Access subscribe methods from `eth` namespace
    pub fn eth_subscribe(&self) -> eth_subscribe::EthSubscribe<T> {
        self.api()
    }
}
