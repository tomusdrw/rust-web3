mod block;
mod bytes;
mod transaction_request;

pub use self::block::{BlockId, BlockNumber};
pub use self::bytes::Bytes;
pub use self::transaction_request::TransactionRequest;

pub type Address = String;
pub type U256 = String;
pub type H256 = String;
