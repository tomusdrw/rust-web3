use crate::types::{Address, Bytes, CallRequest, H256, U256};
use serde::{Deserialize, Serialize};

/// Struct representing signed data returned from `Accounts::sign` method.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct SignedData {
    /// The original message that was signed.
    pub message: Vec<u8>,
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

/// Transaction data for signing.
///
/// The `Accounts::sign_transaction` method will fill optional fields with sane
/// defaults when they are omitted. Specifically the signing account's current
/// transaction count will be used for the `nonce`, the estimated recommended
/// gas price will be used for `gas_price`, and the current network ID will be
/// used for the `chain_id`.
///
/// It is worth noting that the chain ID is not equivalent to the network ID.
/// They happen to be the same much of the time but it is recommended to set
/// this for signing transactions.
///
/// `TransactionParameters` implements `Default` and uses `100_000` as the
/// default `gas` to use for the transaction. This is more than enough for
/// simple transactions sending ETH between accounts but may not be enough when
/// interacting with complex contracts. It is recommended when interacting
/// with contracts to use `Eth::estimate_gas` to estimate the required gas for
/// the transaction.
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
    /// The chain ID (None for network ID)
    pub chain_id: Option<u64>,
}

/// The default fas for transactions.
///
/// Unfortunatly there is no way to construct `U256`s with const functions for
/// constants so we just build it from it's `u64` words. Note that there is a
/// unit test to verify that it is constructed correctly and holds the expected
/// value of 100_000.
const TRANSACTION_DEFAULT_GAS: U256 = U256([100_000, 0, 0, 0]);

impl Default for TransactionParameters {
    fn default() -> Self {
        TransactionParameters {
            nonce: None,
            to: None,
            gas: TRANSACTION_DEFAULT_GAS,
            gas_price: None,
            value: U256::zero(),
            data: Bytes::default(),
            chain_id: None,
        }
    }
}

impl From<CallRequest> for TransactionParameters {
    fn from(call: CallRequest) -> Self {
        let to = if call.to != Address::zero() {
            Some(call.to)
        } else {
            None
        };

        TransactionParameters {
            nonce: None,
            to,
            gas: call.gas.unwrap_or(TRANSACTION_DEFAULT_GAS),
            gas_price: call.gas_price,
            value: call.value.unwrap_or_default(),
            data: call.data.unwrap_or_default(),
            chain_id: None,
        }
    }
}

impl Into<CallRequest> for TransactionParameters {
    fn into(self) -> CallRequest {
        CallRequest {
            from: None,
            to: self.to.unwrap_or_default(),
            gas: Some(self.gas),
            gas_price: self.gas_price,
            value: Some(self.value),
            data: Some(self.data),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_transaction_default_gas() {
        assert_eq!(TRANSACTION_DEFAULT_GAS, U256::from(100_000));
    }
}
