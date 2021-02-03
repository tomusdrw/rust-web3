use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};

use super::{Address, U256, U64};

/// Condition to filter pending transactions
#[derive(Clone, Serialize)]
pub enum FilterCondition<T> {
    /// Lower Than
    #[serde(rename(serialize = "lt"))]
    LowerThan(T),
    /// Equal
    #[serde(rename(serialize = "eq"))]
    Equal(T),
    /// Greater Than
    #[serde(rename(serialize = "gt"))]
    GreaterThan(T),
}

impl<T> From<T> for FilterCondition<T> {
    fn from(t: T) -> Self {
        FilterCondition::Equal(t)
    }
}

/// To Filter
#[derive(Clone)]
pub enum ToFilter {
    /// Address
    Address(Address),
    /// Action (i.e. contract creation)
    Action,
}

/// Filter for pending transactions (only openethereum/Parity)
#[derive(Clone, Default, Serialize)]
pub struct ParityPendingTransactionFilter {
    /// From address
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<FilterCondition<Address>>,
    /// To address or action, i.e. contract creation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<ToFilter>,
    /// Gas
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas: Option<FilterCondition<U64>>,
    /// Gas Price
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gas_price: Option<FilterCondition<U64>>,
    /// Value
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<FilterCondition<U256>>,
    /// Nonce
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<FilterCondition<U256>>,
}

impl Serialize for ToFilter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;

        match self {
            Self::Address(a) => map.serialize_entry("eq", a)?,
            Self::Action => map.serialize_entry("action", "contract_creation")?,
        }
        map.end()
    }
}

impl ParityPendingTransactionFilter {
    /// Returns a filter builder
    pub fn builder() -> ParityPendingTransactionFilterBuilder {
        Default::default()
    }
}

/// Filter Builder
#[derive(Default, Clone)]
pub struct ParityPendingTransactionFilterBuilder {
    filter: ParityPendingTransactionFilter,
}

impl ParityPendingTransactionFilterBuilder {
    /// Sets `from`
    pub fn from(mut self, from: Address) -> Self {
        self.filter.from = Some(FilterCondition::Equal(from));
        self
    }

    /// Sets `to`
    pub fn to(mut self, to_or_action: ToFilter) -> Self {
        self.filter.to = Some(to_or_action);
        self
    }

    /// Sets `gas`
    pub fn gas(mut self, gas: impl Into<FilterCondition<U64>>) -> Self {
        self.filter.gas = Some(gas.into());
        self
    }

    /// Sets `gas_price`
    pub fn gas_price(mut self, gas_price: impl Into<FilterCondition<U64>>) -> Self {
        self.filter.gas_price = Some(gas_price.into());
        self
    }

    /// Sets `value`
    pub fn value(mut self, value: impl Into<FilterCondition<U256>>) -> Self {
        self.filter.value = Some(value.into());
        self
    }

    /// Sets `nonce`
    pub fn nonce(mut self, nonce: impl Into<FilterCondition<U256>>) -> Self {
        self.filter.nonce = Some(nonce.into());
        self
    }

    /// Returns filter
    pub fn build(&self) -> ParityPendingTransactionFilter {
        self.filter.clone()
    }
}
