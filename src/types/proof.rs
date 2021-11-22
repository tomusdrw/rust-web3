use crate::types::Bytes;
use ethereum_types::{H256, U256, U64};
use serde::{Deserialize, Serialize};

///Proof struct returned by eth_getProof method
///
/// https://eips.ethereum.org/EIPS/eip-1186
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Proof {
    /// the balance of the account. See eth_getBalance
    pub balance: U64,
    ///  hash of the code of the account
    #[serde(rename = "codeHash")]
    pub code_hash: H256,
    /// nonce of the account. See eth_getTransactionCount
    pub nonce: U64,
    /// SHA3 of the StorageRoot.
    #[serde(rename = "storageHash")]
    pub storage_hash: H256,
    /// Array of rlp-serialized MerkleTree-Nodes, starting with the stateRoot-Node, following the path of the SHA3 (address) as key.
    #[serde(rename = "accountProof")]
    pub account_proof: Vec<Bytes>,
    /// Array of storage-entries as requested
    #[serde(rename = "storageProof")]
    pub storage_proof: Vec<StorageProof>,
}

/// A key-value pair and it's state proof.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct StorageProof {
    /// the requested storage key
    pub key: U256,
    /// the storage value
    pub value: U256,
    /// Array of rlp-serialized MerkleTree-Nodes, starting with the storageHash-Node, following the path of the SHA3 (key) as path.
    pub proof: Vec<Bytes>,
}
