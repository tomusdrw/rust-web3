mod block_id;
mod bytes;
mod transaction_request;

pub use self::block_id::BlockId;
pub use self::bytes::Bytes;
pub use self::transaction_request::TransactionRequest;

pub type Address = String;
pub type BlockNumber = String;
pub type U256 = String;
