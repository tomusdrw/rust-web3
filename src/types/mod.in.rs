mod block;
mod bytes;
mod transaction_id;
mod transaction_request;
mod uint;

pub use self::block::{BlockId, BlockNumber};
pub use self::bytes::Bytes;
pub use self::transaction_id::TransactionId;
pub use self::transaction_request::{TransactionRequest, CallRequest};
pub use self::uint::{H64, H128, H160, H256, H512, U64, U256};

pub type Address = H160;
pub type Index = U64;
// TODO [ToDr]
pub type Transaction = ();
pub type TransactionReceipt = ();
pub type Block = ();
pub type Work = ();
