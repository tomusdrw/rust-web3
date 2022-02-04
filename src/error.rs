//! Web3 Error
use crate::rpc::error::Error as RPCError;
use derive_more::{Display, From};
use serde_json::Error as SerdeError;
use std::io::Error as IoError;

/// Web3 `Result` type.
pub type Result<T = ()> = std::result::Result<T, Error>;

/// Transport-depended error.
#[derive(Display, Debug, Clone, PartialEq)]
pub enum TransportError {
    /// Transport-specific error code.
    #[display(fmt = "code {}", _0)]
    Code(u16),
    /// Arbitrary, developer-readable description of the occurred error.
    #[display(fmt = "{}", _0)]
    Message(String),
}

/// Errors which can occur when attempting to generate resource uri.
#[derive(Debug, Display, From)]
pub enum Error {
    /// server is unreachable
    #[display(fmt = "Server is unreachable")]
    Unreachable,
    /// decoder error
    #[display(fmt = "Decoder error: {}", _0)]
    Decoder(String),
    /// invalid response
    #[display(fmt = "Got invalid response: {}", _0)]
    #[from(ignore)]
    InvalidResponse(String),
    /// transport error
    #[display(fmt = "Transport error: {}" _0)]
    #[from(ignore)]
    Transport(TransportError),
    /// rpc error
    #[display(fmt = "RPC error: {:?}", _0)]
    Rpc(RPCError),
    /// io error
    #[display(fmt = "IO error: {}", _0)]
    Io(IoError),
    /// recovery error
    #[display(fmt = "Recovery error: {}", _0)]
    Recovery(crate::signing::RecoveryError),
    /// web3 internal error
    #[display(fmt = "Internal Web3 error")]
    Internal,
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        use self::Error::*;
        match *self {
            Unreachable | Decoder(_) | InvalidResponse(_) | Transport { .. } | Internal => None,
            Rpc(ref e) => Some(e),
            Io(ref e) => Some(e),
            Recovery(ref e) => Some(e),
        }
    }
}

impl From<SerdeError> for Error {
    fn from(err: SerdeError) -> Self {
        Error::Decoder(format!("{:?}", err))
    }
}

impl Clone for Error {
    fn clone(&self) -> Self {
        use self::Error::*;
        match self {
            Unreachable => Unreachable,
            Decoder(s) => Decoder(s.clone()),
            InvalidResponse(s) => InvalidResponse(s.clone()),
            Transport(s) => Transport(s.clone()),
            Rpc(e) => Rpc(e.clone()),
            Io(e) => Io(IoError::from(e.kind())),
            Recovery(e) => Recovery(e.clone()),
            Internal => Internal,
        }
    }
}

#[cfg(test)]
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        use self::Error::*;
        match (self, other) {
            (Unreachable, Unreachable) | (Internal, Internal) => true,
            (Decoder(a), Decoder(b)) | (InvalidResponse(a), InvalidResponse(b)) => a == b,
            (Transport(a), Transport(b)) => a == b,
            (Rpc(a), Rpc(b)) => a == b,
            (Io(a), Io(b)) => a.kind() == b.kind(),
            (Recovery(a), Recovery(b)) => a == b,
            _ => false,
        }
    }
}
