mod block;
mod bytes;
mod transaction_id;
mod transaction_request;

pub use self::block::{BlockId, BlockNumber};
pub use self::bytes::Bytes;
pub use self::transaction_id::TransactionId;
pub use self::transaction_request::{TransactionRequest, CallRequest};

pub type Address = String;
pub type H64 = String;
pub type H256 = String;
pub type H512 = String;
pub type Index = String;
pub type U256 = String;
// TODO [ToDr]
pub type Transaction = ();
pub type TransactionReceipt = ();
pub type Block = ();
pub type Work = ();
