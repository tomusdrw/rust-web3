//! Contract call/query error.

use ethabi::Error as EthError;

use crate::error::Error as ApiError;
use derive_more::Display;

/// Contract error.
#[derive(Debug, Display)]
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
}

impl From<EthError> for Error {
    fn from(e: EthError) -> Self {
        Error::Abi(e)
    }
}

impl From<ApiError> for Error {
    fn from(e: ApiError) -> Self {
        Error::Api(e)
    }
}

pub mod deploy {
    use crate::error::Error as ApiError;
    use crate::types::H256;
    use derive_more::Display;

    /// Contract deployment error.
    #[derive(Debug, Display)]
    pub enum Error {
        /// Rpc error
        #[display(fmt = "Api error: {}", _0)]
        Api(ApiError),
        /// Contract deployment failed
        #[display(fmt = "Failure during deployment.Tx hash: {:?}", _0)]
        ContractDeploymentFailure(H256),
    }

    impl From<ApiError> for Error {
        fn from(e: ApiError) -> Self {
            Error::Api(e)
        }
    }
}
