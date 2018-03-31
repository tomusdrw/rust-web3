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

pub use self::block::{Block, BlockHeader, BlockId, BlockNumber};
pub use self::bytes::Bytes;
pub use self::log::{Filter, FilterBuilder, Log};
pub use self::sync_state::{SyncInfo, SyncState};
pub use self::transaction::{Receipt as TransactionReceipt, Transaction};
pub use self::transaction_id::TransactionId;
pub use self::transaction_request::{CallRequest, TransactionCondition, TransactionRequest};
pub use self::uint::{H128, H160, H2048, H256, H512, H520, H64, U128, U256, U64};
pub use self::work::Work;

/// Address
pub type Address = H160;
/// Index in block
pub type Index = U128;
