use crate::types::{Address, Bytes, H256, U256};
use serde::{Deserialize, Serialize};

/// Struct representing signed data returned from `Accounts::sign` method.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SignedData {
    /// The original method that was signed.
    pub message: String,
    /// The keccak256 hash of the signed data.
    #[serde(rename = "messageHash")]
    pub message_hash: H256,
    /// V value.
    pub v: u8,
    /// R value.
    pub r: H256,
    /// S value.
    pub s: H256,
    /// The signature bytes.
    pub signature: Bytes,
}

/// Transaction data for signing. The `Accounts::sign_transaction` method will
/// fill optional fields with sane defaults when they are ommited.
#[derive(Clone, Debug, PartialEq)]
pub struct TransactionParameters {
    /// Transaction nonce (None for account transaction count)
    pub nonce: Option<U256>,
    /// To address
    pub to: Option<Address>,
    /// Supplied gas
    pub gas: U256,
    /// Gas price (None for estimated gas price)
    pub gas_price: Option<U256>,
    /// Transfered value
    pub value: U256,
    /// Data
    pub data: Bytes,
}

impl Default for TransactionParameters {
    fn default() -> Self {
        TransactionParameters {
            nonce: None,
            to: None,
            gas: 100_000.into(),
            gas_price: None,
            value: U256::zero(),
            data: Bytes::default(),
        }
    }
}

/// Data for offline signed transaction
#[derive(Clone, Debug, PartialEq)]
pub struct SignedTransaction {
    /// The given message hash
    pub message_hash: H256,
    /// V value with chain replay protection.
    pub v: u64,
    /// R value.
    pub r: H256,
    /// S value.
    pub s: H256,
    /// The raw signed transaction ready to be sent with `send_raw_transaction`
    pub raw_transaction: Bytes,
    /// The transaction hash for the RLP encoded transaction.
    pub transaction_hash: H256,
}
