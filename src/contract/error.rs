//! Contract call/query error.

use crate::error::Error as ApiError;
use derive_more::{Display, From};
use ethabi::Error as EthError;

/// Contract error.
#[derive(Debug, Display, From)]
pub enum Error {
    /// invalid output type requested by the caller
    #[display(fmt = "Invalid output type: {}", _0)]
    InvalidOutputType(String),
    /// eth abi error
    #[display(fmt = "Abi error: {}", _0)]
    Abi(EthError),
    /// Rpc error
    #[display(fmt = "Api error: {}", _0)]
    Api(ApiError),
    /// An error during deployment.
    #[display(fmt = "Deployment error: {}", _0)]
    Deployment(crate::contract::deploy::Error),
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::InvalidOutputType(_) => None,
            Error::Abi(ref e) => Some(e),
            Error::Api(ref e) => Some(e),
            Error::Deployment(ref e) => Some(e),
        }
    }
}

pub mod deploy {
    use crate::{error::Error as ApiError, types::H256};
    use derive_more::{Display, From};

    /// Contract deployment error.
    #[derive(Debug, Display, From)]
    pub enum Error {
        /// eth abi error
        #[display(fmt = "Abi error: {}", _0)]
        Abi(ethabi::Error),
        /// Rpc error
        #[display(fmt = "Api error: {}", _0)]
        Api(ApiError),
        /// Contract deployment failed
        #[display(fmt = "Failure during deployment.Tx hash: {:?}", _0)]
        ContractDeploymentFailure(H256),
    }

    impl std::error::Error for Error {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match *self {
                Error::Abi(ref e) => Some(e),
                Error::Api(ref e) => Some(e),
                Error::ContractDeploymentFailure(_) => None,
            }
        }
    }
}
