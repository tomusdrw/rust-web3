//! Web3 Types

mod block;
mod bytes;
mod bytes_array;
mod fee_history;
mod log;
mod parity_peers;
mod parity_pending_transaction;
mod proof;
mod recovery;
mod signed;
mod sync_state;
mod trace_filtering;
mod traces;
mod transaction;
mod transaction_id;
mod transaction_request;
mod txpool;
mod uint;
mod work;

pub use self::{
    block::{Block, BlockHeader, BlockId, BlockNumber},
    bytes::Bytes,
    bytes_array::BytesArray,
    fee_history::FeeHistory,
    log::{Filter, FilterBuilder, Log},
    parity_peers::{
        EthProtocolInfo, ParityPeerInfo, ParityPeerType, PeerNetworkInfo, PeerProtocolsInfo, PipProtocolInfo,
    },
    parity_pending_transaction::{
        FilterCondition, ParityPendingTransactionFilter, ParityPendingTransactionFilterBuilder, ToFilter,
    },
    proof::Proof,
    recovery::{ParseSignatureError, Recovery, RecoveryMessage},
    signed::{SignedData, SignedTransaction, TransactionParameters},
    sync_state::{SyncInfo, SyncState},
    trace_filtering::{
        Action, ActionType, Call, CallResult, CallType, Create, CreateResult, Res, Reward, RewardType, Suicide, Trace,
        TraceFilter, TraceFilterBuilder,
    },
    traces::{
        AccountDiff, BlockTrace, ChangedType, Diff, MemoryDiff, StateDiff, StorageDiff, TraceType, TransactionTrace,
        VMExecutedOperation, VMOperation, VMTrace,
    },
    transaction::{AccessList, AccessListItem, RawTransaction, Receipt as TransactionReceipt, Transaction},
    transaction_id::TransactionId,
    transaction_request::{CallRequest, TransactionCondition, TransactionRequest},
    txpool::{TxpoolContentInfo, TxpoolInspectInfo, TxpoolStatus},
    uint::{H128, H160, H2048, H256, H512, H520, H64, U128, U256, U64},
    work::Work,
};

/// Address
pub type Address = H160;
/// Index in block
pub type Index = U64;
