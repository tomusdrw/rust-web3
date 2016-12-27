use types::{Address, U256, Bytes};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CallRequest {
  #[serde(skip_serializing_if="Option::is_none")]
  pub from: Option<Address>,
  pub to: Address,
  #[serde(skip_serializing_if="Option::is_none")]
  pub gas: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  #[serde(rename = "gasPrice")]
  pub gas_price: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub value: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub data: Option<Bytes>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TransactionRequest {
  pub from: Address,
  #[serde(skip_serializing_if="Option::is_none")]
  pub to: Option<Address>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub gas: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  #[serde(rename = "gasPrice")]
  pub gas_price: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub value: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub data: Option<Bytes>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub nonce: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  #[serde(rename = "minBlock")]
  pub min_block: Option<U256>,
}

// TODO serialization test
