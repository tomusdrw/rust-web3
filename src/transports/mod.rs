//! Supported Ethereum JSON-RPC transports.

pub mod batch;
pub use self::batch::Batch;
pub mod either;
pub use self::either::Either;

#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "http")]
pub use self::http::Http;

#[cfg(any(feature = "ws-tokio", feature = "ws-async-std"))]
pub mod ws;
#[cfg(any(feature = "ws-tokio", feature = "ws-async-std"))]
pub use self::ws::WebSocket;

#[cfg(feature = "ipc-tokio")]
pub mod ipc;
#[cfg(feature = "ipc-tokio")]
pub use self::ipc::Ipc;

#[cfg(any(feature = "test", test))]
pub mod test;

#[cfg(feature = "url")]
impl From<url::ParseError> for crate::Error {
    fn from(err: url::ParseError) -> Self {
        crate::Error::Transport(format!("{:?}", err))
    }
}

#[cfg(feature = "async-native-tls")]
impl From<async_native_tls::Error> for crate::Error {
    fn from(err: async_native_tls::Error) -> Self {
        crate::Error::Transport(format!("{:?}", err))
    }
}

#[cfg(feature = "eip-1193")]
pub mod eip_1193;
