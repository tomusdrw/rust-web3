//! Type of node (Parity, Geth...)

/// specify behavior of node interacting with
pub enum NodeType {
    /// parity node
    Parity,
    /// geth node
    Geth,
}
