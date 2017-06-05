//! Supported Ethereum JSON-RPC transports.

use {Error};

/// RPC Result.
pub type Result<T> = ::std::result::Result<T, Error>;

pub mod batch;
pub use self::batch::Batch;

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "http")]
pub use self::http::Http;

#[cfg(feature = "ipc")]
pub mod ipc;
#[cfg(feature = "ipc")]
pub use self::ipc::Ipc;

