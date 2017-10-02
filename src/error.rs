//! Web3 Error

#![allow(unknown_lints)]
#![allow(missing_docs)]

use std::io;
use serde_json;
use rpc;

error_chain! {
  foreign_links {
    Io(io::Error);
  }
  errors {
    Unreachable {
      description("server is unreachable"),
      display("Server is unreachable"),
    }
    Decoder(e: String) {
      description("decoder error"),
      display("Decoder error: {}", e),
    }
    InvalidResponse(e: String) {
      description("invalid response"),
      display("Got invalid response: {}", e),
    }
    Transport(e: String) {
      description("transport error"),
      display("Transport error: {}", e),
    }
    // TODO [ToDr] Move to foreign_links
    Rpc(e: rpc::Error) {
      description("rpc error"),
      display("RPC error: {:?}", e),
    }
    Internal {
      description("web3 internal error"),
      display("Internal Web3 error"),
    }
  }
}

impl From<serde_json::Error> for Error {
  fn from(err: serde_json::Error) -> Self {
    ErrorKind::Decoder(format!("{:?}", err)).into()
  }
}

impl Clone for Error {
  fn clone(&self) -> Self {
    match *self.kind() {
      ErrorKind::Io(ref io) => ErrorKind::Io(io.kind().clone().into()),
      ErrorKind::Unreachable => ErrorKind::Unreachable,
      ErrorKind::Decoder(ref err) => ErrorKind::Decoder(err.to_owned()),
      ErrorKind::InvalidResponse(ref t) => ErrorKind::InvalidResponse(t.to_owned()),
      ErrorKind::Transport(ref t) => ErrorKind::Transport(t.to_owned()),
      ErrorKind::Rpc(ref e) => ErrorKind::Rpc(e.clone()),
      ErrorKind::Internal => ErrorKind::Internal,
      ErrorKind::Msg(ref e) => ErrorKind::Msg(e.clone()).into(),
      _ => unimplemented!(),
    }.into()
  }
}

#[cfg(test)]
impl PartialEq for Error {
  fn eq(&self, o: &Self) -> bool {
    *self.kind() == *o.kind()
  }
}

#[cfg(test)]
impl PartialEq for ErrorKind {
  fn eq(&self, o: &Self) -> bool {
    use self::ErrorKind::*;

    match (self, o) {
      (&Io(_), &Io(_)) => true,
      (&Unreachable, &Unreachable) => true,
      (&Decoder(ref a), &Decoder(ref b)) => a == b,
      (&InvalidResponse(ref a), &InvalidResponse(ref b)) => a == b,
      (&Transport(ref a), &Transport(ref b)) => a == b,
      (&Rpc(ref a), &Rpc(ref b)) => a == b,
      (&Internal, &Internal) => true,
      (&Msg(ref a), &Msg(ref b)) => a == b,
      _ => false,
    }
  }
}
