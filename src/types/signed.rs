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
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransactionParameters {
    /// Transaction nonce (None for account transaction count)
    pub nonce: Option<U256>,
    /// To address
    pub to: Option<Address>,
    /// Supplied gas (None for sensible default)
    pub gas: Option<U256>,
    /// Gas price (None for sensible default)
    pub gas_price: Option<U256>,
    /// Transfered value (None for no transfer)
    pub value: Option<U256>,
    /// Data (None for empty data)
    pub data: Option<Bytes>,
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
