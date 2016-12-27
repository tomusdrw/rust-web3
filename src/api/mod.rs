//! `Web3` implementation

pub mod eth;
use {Transport};

/// `Web3` wrapper for all namespaces
pub struct Web3<T: Transport> {
  transport: T,
}

impl<T: Transport> Web3<T> {
  /// Create new `Web3` with given transport
  pub fn new(transport: T) -> Self {
    Web3 {
      transport: transport,
    }
  }

  /// Access methods from `eth` namespace
  pub fn eth(&self) -> eth::Eth<T> {
    eth::Eth::new(&self.transport)
  }

  /// Access filter methods from `eth` namespace
  pub fn eth_filter(&self) -> ! {
    unimplemented!()
  }
}
