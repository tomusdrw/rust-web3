use crate::types::{Bytes, H256};
use serde::{Deserialize, Serialize};

/// Struct representing signed data returned from `Accounts::sign` method.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SignedData {
    /// The original method that was signed.
    pub message: String,
    /// The keccak256 hash of the signed data.
    #[serde(rename = "messageHash")]
    pub message_hash: H256,
    /// V value.
    pub v: u8,
    /// R value.
    pub r: [u8; 32],
    /// S value.
    pub s: [u8; 32],
    /// The signature bytes.
    pub signature: Bytes,
}
