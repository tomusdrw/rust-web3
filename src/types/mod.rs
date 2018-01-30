//! Web3 Types

mod block;
mod bytes;
mod log;
mod sync_state;
mod transaction;
mod transaction_id;
mod transaction_request;
mod uint;
mod work;

pub use self::block::{Block, BlockId, BlockNumber};
pub use self::bytes::Bytes;
pub use self::log::{Log, Filter, FilterBuilder};
pub use self::sync_state::{SyncState,SyncInfo};
pub use self::transaction::{Transaction, Receipt as TransactionReceipt};
pub use self::transaction_id::TransactionId;
pub use self::transaction_request::{TransactionRequest, CallRequest, TransactionCondition};
pub use self::uint::{H64, H128, H160, H256, H512, H520, H2048, U64, U256};
pub use self::work::Work;

/// Address
pub type Address = H160;
/// Index in block
pub type Index = U64;
