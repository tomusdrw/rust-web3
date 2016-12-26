use types::{Address, U256, Bytes};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct TransactionRequest {
  #[serde(skip_serializing_if="Option::is_none")]
  pub from: Option<Address>,
  pub to: Address,
  #[serde(skip_serializing_if="Option::is_none")]
  pub gas: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub gas_price: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub value: Option<U256>,
  #[serde(skip_serializing_if="Option::is_none")]
  pub data: Option<Bytes>,
}

