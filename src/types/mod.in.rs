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

/// Address
pub type Address = H160;
/// Index in block
pub type Index = U64;
/// TODO [ToDr] Transaction
pub type Transaction = ();
/// TODO [ToDr] Transaction Receipt
pub type TransactionReceipt = ();
/// TODO [ToDr] Block
pub type Block = ();
/// TODO [ToDr] Work
pub type Work = ();
