//! Contract call/query error.

#![allow(unknown_lints)]
#![allow(missing_docs)]

use ethabi;

error_chain! {
  links {
    Abi(ethabi::Error, ethabi::ErrorKind);
    Api(::Error, ::ErrorKind);
  }

  errors {
    InvalidOutputType(e: String) {
      description("invalid output type requested by the caller"),
      display("Invalid output type: {}", e),
    }
  }
}

/// Contract deployment error.
pub mod deploy {
    use types::H256;

    error_chain! {
      links {
        Api(::Error, ::ErrorKind);
      }

      errors {
        ContractDeploymentFailure(hash: H256) {
          description("Contract deployment failed")
          display("Failure during deployment. Tx hash: {:?}", hash),
        }
      }
    }
}
