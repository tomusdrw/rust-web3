use crate::types::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

/// Call contract request (eth_call / eth_estimateGas)
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CallRequest {
    /// Sender address (None for arbitrary address)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Address>,
    /// To address
    pub to: Address,
    /// Supplied gas (None for sensible default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<U256>,
    /// Gas price (None for sensible default)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<U256>,
    /// Transfered value (None for no transfer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// Data (None for empty data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
}

/// Send Transaction Parameters
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TransactionRequest {
    /// Sender address
    pub from: Address,
    /// Recipient address (None for contract creation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Address>,
    /// Supplied gas (None for sensible default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<U256>,
    /// Gas price (None for sensible default)
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "gasPrice")]
    pub gas_price: Option<U256>,
    /// Transfered value (None for no transfer)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<U256>,
    /// Transaction data (None for empty bytes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
    /// Transaction nonce (None for next available nonce)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<U256>,
    /// Min block inclusion (None for include immediately)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<TransactionCondition>,
}

/// Represents condition on minimum block number or block timestamp.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum TransactionCondition {
    /// Valid at this minimum block number.
    #[serde(rename = "block")]
    Block(u64),
    /// Valid at given unix time.
    #[serde(rename = "time")]
    Timestamp(u64),
}

#[cfg(test)]
mod tests {
    use super::{Address, CallRequest, TransactionCondition, TransactionRequest};
    use serde_json;

    #[test]
    fn should_serialize_call_request() {
        // given
        let call_request = CallRequest {
            from: None,
            to: Address::from_low_u64_be(5),
            gas: Some(21_000.into()),
            gas_price: None,
            value: Some(5_000_000.into()),
            data: Some(vec![1, 2, 3].into()),
        };

        // when
        let serialized = serde_json::to_string_pretty(&call_request).unwrap();

        // then
        assert_eq!(
            serialized,
            r#"{
  "to": "0x0000000000000000000000000000000000000005",
  "gas": "0x5208",
  "value": "0x4c4b40",
  "data": "0x010203"
}"#
        );
    }

    #[test]
    fn should_serialize_transaction_request() {
        // given
        let tx_request = TransactionRequest {
            from: Address::from_low_u64_be(5),
            to: None,
            gas: Some(21_000.into()),
            gas_price: None,
            value: Some(5_000_000.into()),
            data: Some(vec![1, 2, 3].into()),
            nonce: None,
            condition: Some(TransactionCondition::Block(5)),
        };

        // when
        let serialized = serde_json::to_string_pretty(&tx_request).unwrap();

        // then
        assert_eq!(
            serialized,
            r#"{
  "from": "0x0000000000000000000000000000000000000005",
  "gas": "0x5208",
  "value": "0x4c4b40",
  "data": "0x010203",
  "condition": {
    "block": 5
  }
}"#
        );
    }
}
