//! Supported Ethereum JSON-RPC transports.

use Error;

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

#[cfg(feature = "ws")]
pub mod ws;
#[cfg(feature = "ws")]
pub use self::ws::WebSocket;

#[cfg(any(feature = "ipc", feature = "http", feature = "ws"))]
mod shared;
#[cfg(any(feature = "ipc", feature = "http", feature = "ws"))]
extern crate tokio_core;
#[cfg(any(feature = "ipc"))]
extern crate tokio_io;
#[cfg(any(feature = "ipc", feature = "http", feature = "ws"))]
pub use self::shared::EventLoopHandle;
