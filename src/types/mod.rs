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
mod traces;
mod trace_filtering;
mod parity_peers;

pub use self::block::{Block, BlockHeader, BlockId, BlockNumber};
pub use self::bytes::Bytes;
pub use self::log::{Filter, FilterBuilder, Log};
pub use self::sync_state::{SyncInfo, SyncState};
pub use self::transaction::{Receipt as TransactionReceipt, Transaction, RawTransaction};
pub use self::transaction_id::TransactionId;
pub use self::transaction_request::{CallRequest, TransactionCondition, TransactionRequest};
pub use self::uint::{H128, H160, H2048, H256, H512, H520, H64, U128, U256, U64};
pub use self::work::Work;
pub use self::trace_filtering::{Trace, TraceFilter, TraceFilterBuilder, Res, Action, CallType};
pub use self::traces::{TraceType, BlockTrace, TransactionTrace};
pub use self::parity_peers::{ParityPeerType, ParityPeerInfo, PeerNetworkInfo, PeerProtocolsInfo};

/// Address
pub type Address = H160;
/// Index in block
pub type Index = U128;
