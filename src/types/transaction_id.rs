use types::{BlockId, H256, Index};

/// Transaction Identifier
#[derive(Clone, Debug, PartialEq)]
pub enum TransactionId {
    /// By hash
    Hash(H256),
    /// By block and index
    Block(BlockId, Index),
}
