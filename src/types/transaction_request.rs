use types::{Address, U256, Bytes};

/// Call contract request (eth_call / eth_estimateGas)
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CallRequest {
  /// Sender address (None for arbitrary address)
  #[serde(skip_serializing_if="Option::is_none")]
  pub from: Option<Address>,
  /// To address
  pub to: Address,
  /// Supplied gas (None for sensible default)
  #[serde(skip_serializing_if="Option::is_none")]
  pub gas: Option<U256>,
  /// Gas price (None for sensible default)
  #[serde(skip_serializing_if="Option::is_none")]
  #[serde(rename = "gasPrice")]
  pub gas_price: Option<U256>,
  /// Transfered value (None for no transfer)
  #[serde(skip_serializing_if="Option::is_none")]
  pub value: Option<U256>,
  /// Data (None for empty data)
  #[serde(skip_serializing_if="Option::is_none")]
  pub data: Option<Bytes>,
}

/// Send Transaction Parameters
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TransactionRequest {
  /// Sender address
  pub from: Address,
  /// Recipient address (None for contract creation)
  #[serde(skip_serializing_if="Option::is_none")]
  pub to: Option<Address>,
  /// Supplied gas (None for sensible default)
  #[serde(skip_serializing_if="Option::is_none")]
  pub gas: Option<U256>,
  /// Gas price (None for sensible default)
  #[serde(skip_serializing_if="Option::is_none")]
  #[serde(rename = "gasPrice")]
  pub gas_price: Option<U256>,
  /// Transfered value (None for no transfer)
  #[serde(skip_serializing_if="Option::is_none")]
  pub value: Option<U256>,
  /// Transaction data (None for empty bytes)
  #[serde(skip_serializing_if="Option::is_none")]
  pub data: Option<Bytes>,
  /// Transaction nonce (None for next available nonce)
  #[serde(skip_serializing_if="Option::is_none")]
  pub nonce: Option<U256>,
  /// Min block inclusion (None for include immediately)
  #[serde(skip_serializing_if="Option::is_none")]
  pub condition: Option<Condition>,
}

/// Represents condition on minimum block number or block timestamp.
#[derive(Debug, Clone, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum Condition {
	/// Valid at this minimum block number.
	#[serde(rename="block")]
	Block(u64),
	/// Valid at given unix time.
	#[serde(rename="time")]
	Timestamp(u64),
}

// TODO serialization test
