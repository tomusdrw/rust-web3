use types::{BlockId, H256, Index};

#[derive(Clone, Debug, PartialEq)]
pub enum TransactionId {
  Hash(H256),
  Block(BlockId, Index),
}
