use crate::types::{BlockId, Index, H256};

/// Transaction Identifier
#[derive(Clone, Debug, PartialEq)]
pub enum TransactionId {
    /// By hash
    Hash(H256),
    /// By block and index
    Block(BlockId, Index),
}
