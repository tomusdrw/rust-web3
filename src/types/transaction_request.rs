use crate::types::{Address, Bytes, U256};
use serde::{Deserialize, Serialize};

/// Call contract request (eth_call / eth_estimateGas)
///
/// When using this for `eth_estimateGas`, all the fields
/// are optional. However, for usage in `eth_call` the
/// `to` field must be provided.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct CallRequest {
    /// Sender address (None for arbitrary address)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Address>,
    /// To address (None allowed for eth_estimateGas)
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
    /// Data (None for empty data)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Bytes>,
}

/// Send Transaction Parameters
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
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
    use hex_literal::hex;

    use super::{Address, CallRequest, TransactionCondition, TransactionRequest};

    #[test]
    fn should_serialize_call_request() {
        // given
        let call_request = CallRequest {
            from: None,
            to: Some(Address::from_low_u64_be(5)),
            gas: Some(21_000.into()),
            gas_price: None,
            value: Some(5_000_000.into()),
            data: Some(hex!("010203").into()),
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
    fn should_deserialize_call_request() {
        let serialized = r#"{
  "to": "0x0000000000000000000000000000000000000005",
  "gas": "0x5208",
  "value": "0x4c4b40",
  "data": "0x010203"
}"#;
        let deserialized: CallRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.from, None);
        assert_eq!(deserialized.to, Some(Address::from_low_u64_be(5)));
        assert_eq!(deserialized.gas, Some(21_000.into()));
        assert_eq!(deserialized.gas_price, None);
        assert_eq!(deserialized.value, Some(5_000_000.into()));
        assert_eq!(deserialized.data, Some(hex!("010203").into()));
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
            data: Some(hex!("010203").into()),
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

    #[test]
    fn should_deserialize_transaction_request() {
        let serialized = r#"{
  "from": "0x0000000000000000000000000000000000000005",
  "gas": "0x5208",
  "value": "0x4c4b40",
  "data": "0x010203",
  "condition": {
    "block": 5
  }
}"#;
        let deserialized: TransactionRequest = serde_json::from_str(&serialized).unwrap();

        assert_eq!(deserialized.from, Address::from_low_u64_be(5));
        assert_eq!(deserialized.to, None);
        assert_eq!(deserialized.gas, Some(21_000.into()));
        assert_eq!(deserialized.gas_price, None);
        assert_eq!(deserialized.value, Some(5_000_000.into()));
        assert_eq!(deserialized.data, Some(hex!("010203").into()));
        assert_eq!(deserialized.nonce, None);
        assert_eq!(deserialized.condition, Some(TransactionCondition::Block(5)));
    }
}
