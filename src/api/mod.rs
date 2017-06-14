//! `Web3` implementation

mod eth;
mod eth_filter;
mod net;
mod personal;
mod web3;

pub use self::eth::Eth;
pub use self::eth_filter::{BaseFilter, CreateFilter, EthFilter};
pub use self::net::Net;
pub use self::personal::Personal;
pub use self::web3::Web3;

use std::time::Duration;
use futures::{Future, IntoFuture};
use {confirm, Transport, Error};
use types::{H256, U256, TransactionRequest, TransactionReceipt};

/// Common API for all namespaces
pub trait Namespace<T: Transport> {
  /// Creates new API namespace
  fn new(transport: T) -> Self where Self: Sized;

  /// Borrows a transport.
  fn transport(&self) -> &T;
}

/// `Web3` wrapper for all namespaces
pub struct Web3Main<T: Transport> {
  transport: T,
}

/// Transport-erased `Web3 wrapper.
/// Create this by calling `Web3Main::new` with a transport you've
/// previously called `erase()` on.
pub type ErasedWeb3 = Web3Main<::Erased>;

impl<T: Transport> Web3Main<T> {
  /// Create new `Web3` with given transport
  pub fn new(transport: T) -> Self {
    Web3Main {
      transport,
    }
  }

  /// Borrows a transport.
  pub fn transport(&self) -> &T {
    &self.transport
  }

  /// Access methods from custom namespace
  pub fn api<'a, A: Namespace<&'a T>>(&'a self) -> A {
    A::new(&self.transport)
  }

  /// Access methods from `eth` namespace
  pub fn eth(&self) -> eth::Eth<&T> {
    self.api()
  }

  /// Access methods from `net` namespace
  pub fn net(&self) -> net::Net<&T> {
    self.api()
  }

  /// Access methods from `web3` namespace
  pub fn web3(&self) -> web3::Web3<&T> {
    self.api()
  }

  /// Access filter methods from `eth` namespace
  pub fn eth_filter(&self) -> eth_filter::EthFilter<&T> {
    self.api()
  }

  /// Access methods from `personal` namespace
  pub fn personal(&self) -> personal::Personal<&T> {
    self.api()
  }

  /// Should be used to wait for confirmations
  pub fn wait_for_confirmations<'a, F, V>(&'a self, poll_interval: Duration, confirmations: usize, check: V)
  -> Box<Future<Item = (), Error = Error> + 'a> where
  F: 'a + IntoFuture<Item = Option<U256>, Error = Error>,
  V: 'a + Fn() -> F,
  {
    confirm::wait_for_confirmations(&self.transport, poll_interval, confirmations, check)
  }


  /// Sends transaction and returns future resolved after transaction is confirmed
  pub fn send_transaction_with_confirmation<'a>(&'a self, tx: TransactionRequest, poll_interval: Duration, confirmations: usize)
  -> Box<Future<Item = TransactionReceipt, Error = Error> + 'a> where
  {
    confirm::send_transaction_with_confirmation(&self.transport, tx, poll_interval, confirmations)
  }
}
