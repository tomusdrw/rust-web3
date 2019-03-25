//! Web3 Error
use crate::rpc::error::Error as RPCError;
use serde_json::Error as SerdeError;
use std::io::Error as IoError;

/// Errors which can occur when attempting to generate resource uri.
#[derive(Debug, Display)]
pub enum Error {
    /// server is unreachable
    #[display(fmt = "Server is unreachable")]
    Unreachable,
    /// decoder error
    #[display(fmt = "Decoder error: {}", _0)]
    Decoder(String),
    /// invalid response
    #[display(fmt = "Got invalid response: {}", _0)]
    InvalidResponse(String),
    /// transport error
    #[display(fmt = "Transport error: {}", _0)]
    Transport(String),
    /// rpc error
    #[display(fmt = "RPC error: {:?}", _0)]
    Rpc(RPCError),
    /// io error
    #[display(fmt = "IO error: {}", _0)]
    Io(IoError),
    /// web3 internal error
    #[display(fmt = "Internal Web3 error")]
    Internal,
}

impl From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error::Io(e)
    }
}

impl From<RPCError> for Error {
    fn from(e: RPCError) -> Self {
        Error::Rpc(e)
    }
}

impl From<SerdeError> for Error {
    fn from(err: SerdeError) -> Self {
        Error::Decoder(format!("{:?}", err)).into()
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
            Io(e) => Io(IoError::from(e.kind().clone())),
            Internal => Internal,
        }
    }
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        use self::Error::*;
        match (self, other) {
            (Unreachable, Unreachable) | (Internal, Internal) => true,
            (Decoder(a), Decoder(b)) | (InvalidResponse(a), InvalidResponse(b)) | (Transport(a), Transport(b)) => a == b,
            (Rpc(a), Rpc(b)) => a == b,
            (Io(a), Io(b)) => a.kind() == b.kind(),
            _ => false,
        }
    }
}
