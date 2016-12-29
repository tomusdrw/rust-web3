mod block;
mod bytes;
mod transaction_id;
mod transaction_request;
mod uint;

pub use self::block::{BlockId, BlockNumber};
pub use self::bytes::Bytes;
pub use self::transaction_id::TransactionId;
pub use self::transaction_request::{TransactionRequest, CallRequest};
pub use self::uint::U256;

pub type Address = String;
pub type H64 = String;
pub type H256 = String;
pub type H512 = String;
pub type Index = String;
// TODO [ToDr]
pub type Transaction = ();
pub type TransactionReceipt = ();
pub type Block = ();
pub type Work = ();
