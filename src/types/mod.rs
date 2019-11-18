//! Web3 Types

mod block;
mod bytes;
mod log;
mod parity_peers;
mod recovery;
mod signed;
mod sync_state;
mod trace_filtering;
mod traces;
mod transaction;
mod transaction_id;
mod transaction_request;
mod uint;
mod work;

pub use self::block::{Block, BlockHeader, BlockId, BlockNumber};
pub use self::bytes::Bytes;
pub use self::log::{Filter, FilterBuilder, Log};
pub use self::parity_peers::{
    EthProtocolInfo, ParityPeerInfo, ParityPeerType, PeerNetworkInfo, PeerProtocolsInfo, PipProtocolInfo,
};
pub use self::recovery::{Recovery, RecoveryMessage};
pub use self::signed::{SignedData, SignedTransaction, TransactionParameters};
pub use self::sync_state::{SyncInfo, SyncState};
pub use self::trace_filtering::{
    Action, ActionType, Call, CallResult, CallType, Create, CreateResult, Res, Reward, RewardType, Suicide, Trace,
    TraceFilter, TraceFilterBuilder,
};
pub use self::traces::{
    AccountDiff, BlockTrace, ChangedType, Diff, MemoryDiff, StateDiff, StorageDiff, TraceType, TransactionTrace,
    VMExecutedOperation, VMOperation, VMTrace,
};
pub use self::transaction::{RawTransaction, Receipt as TransactionReceipt, Transaction};
pub use self::transaction_id::TransactionId;
pub use self::transaction_request::{CallRequest, TransactionCondition, TransactionRequest};
pub use self::uint::{H128, H160, H2048, H256, H512, H520, H64, U128, U256, U64};
pub use self::work::Work;

/// Address
pub type Address = H160;
/// Index in block
pub type Index = U128;
