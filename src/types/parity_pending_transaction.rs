use serde::{
    ser::{SerializeMap, Serializer},
    Serialize,
};

use super::{Address, U256, U64};

/// Condition to filter pending transactions
#[derive(Clone)]
pub enum FilterCondition<T> {
    /// Lower Than
    Lt(T),
    /// Equal
    Eq(T),
    /// Greater Than
    Gt(T),
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

impl<T> Serialize for FilterCondition<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::Lt(v) => map.serialize_entry("lt", v),
            Self::Eq(v) => map.serialize_entry("eq", v),
            Self::Gt(v) => map.serialize_entry("gt", v),
        }?;
        map.end()
    }
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

/// Filter Builder
#[derive(Default, Clone)]
pub struct ParityPendingTransactionFilterBuilder {
    filter: ParityPendingTransactionFilter,
}

impl ParityPendingTransactionFilterBuilder {
    /// Sets `from`
    pub fn from(mut self, from: FilterCondition<Address>) -> Self {
        if let FilterCondition::Eq(_) = from {
            self.filter.from = Some(from);
            self
        } else {
            panic!("Must use FilterConditon::Eq to apply filter for `from` address")
        }
    }

    /// Sets `to`
    pub fn to(mut self, to_or_action: ToFilter) -> Self {
        self.filter.to = Some(to_or_action);
        self
    }

    /// Sets `gas`
    pub fn gas(mut self, gas: FilterCondition<U64>) -> Self {
        self.filter.gas = Some(gas);
        self
    }

    /// Sets `gas_price`
    pub fn gas_price(mut self, gas_price: FilterCondition<U64>) -> Self {
        self.filter.gas_price = Some(gas_price);
        self
    }

    /// Sets `value`
    pub fn value(mut self, value: FilterCondition<U256>) -> Self {
        self.filter.value = Some(value);
        self
    }

    /// Sets `nonce`
    pub fn nonce(mut self, nonce: FilterCondition<U256>) -> Self {
        self.filter.nonce = Some(nonce);
        self
    }

    /// Returns filter
    pub fn build(&self) -> ParityPendingTransactionFilter {
        self.filter.clone()
    }
}
