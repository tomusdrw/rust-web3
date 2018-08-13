//! Types for the Parity Ad-Hoc Trace API
use types::{H160, H256, U256, Bytes, Action, Res};
use std::fmt;
use std::collections::BTreeMap;
use serde_json::{self, value};
use serde::de::{self, Deserialize, Deserializer, Visitor, MapAccess};

#[derive(Debug, Clone, Serialize)]
/// Description of the type of trace to make
pub enum TraceType {
    /// Transaction Trace
    #[serde(rename = "trace")]
    Trace,
    /// Virtual Machine Execution Trace
    #[serde(rename = "vmTrace")]
    VmTrace,
    /// State Difference
    #[serde(rename = "stateDiff")]
    StateDiff,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
/// Ad-Hoc trace API type
pub struct BlockTrace {
    /// Output Bytes
    pub output: Bytes,
    /// Transaction Trace
    pub trace: Option<Vec<TransactionTrace>>,
    /// Virtual Machine Execution Trace
    #[serde(rename = "vmTrace")]
    pub vm_trace: Option<VMTrace>,
    /// State Difference
    #[serde(rename = "stateDiff")]
    pub state_diff: Option<StateDiff>,
}

//---------------- State Diff ----------------
#[derive(Debug, PartialEq, Clone, Deserialize)]
/// Aux type for Diff::Changed.
pub struct ChangedType<T> {
    from: T,
    to: T,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
/// Serde-friendly `Diff` shadow.
pub enum Diff<T> {
    #[serde(rename="=")]
    Same,
    #[serde(rename="+")]
    Born(T),
    #[serde(rename="-")]
    Died(T),
    #[serde(rename="*")]
    Changed(ChangedType<T>),
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
/// Serde-friendly `AccountDiff` shadow.
pub struct AccountDiff {
    pub balance: Diff<U256>,
    pub nonce: Diff<U256>,
    pub code: Diff<Bytes>,
    pub storage: BTreeMap<H256, Diff<H256>>,
}

#[derive(Debug, PartialEq, Clone, Deserialize)]
/// Serde-friendly `StateDiff` shadow.
pub struct StateDiff(BTreeMap<H160, AccountDiff>);

// ------------------ Trace -------------
/// Trace
#[derive(Debug, PartialEq, Clone)]
pub struct TransactionTrace {
	/// Trace address
	trace_address: Vec<usize>,
	/// Subtraces
	subtraces: usize,
	/// Action
	action: Action,
	/// Result
	result: Option<Res>,
}

macro_rules! de_value {
    ($action: ident) => ({
        serde_json::from_value($action).map_err(|e| de::Error::custom(e.to_string()))
    })
}
// a pretty standard custom deserialize, except it deserializes 'error' and 'result' of JSON 
// into the result enum, as well as deserializes `Action` based upon `type` field of the JSON.
impl<'de> Deserialize<'de> for TransactionTrace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum TxType { Call, Create, Suicide, Reward };

        enum Field {
            // #[serde(rename = "trace_address")]
            TraceAddress,
            Subtraces,
            Action,
            // #[serde(rename = "result")]
            Result,
            // #[serde(rename = "transaction_type")]
            TransactionType
        };

        // need custom impl, because Result can either be in `result` field or `error` field of JSON
        struct TraceVisitor;
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error> 
                where
                    D: Deserializer<'de>,
            {
                struct FieldVisitor;
                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`trace_address`, `subtraces`, `action`, or `result`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where
                            E: de::Error,
                    {
                        match value {
                            "traceAddress" => Ok(Field::TraceAddress),
                            "subtraces" => Ok(Field::Subtraces),
                            "action" => Ok(Field::Action),
                            "type" => Ok(Field::TransactionType),
                            "error" => Ok(Field::Result),
                            "result" => Ok(Field::Result),
                            _ => Err(de::Error::unknown_field(value, FIELDS))
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        impl<'de> Visitor<'de> for TraceVisitor {
            type Value = TransactionTrace;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Ad-Hoc Trace struct")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
                where
                    M: MapAccess<'de>
            {
                let mut trace_address = None;
                let mut subtraces = None;
                let mut action: Option<value::Value> = None;
                let mut result = None;
                let mut tx_type: Option<TxType> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::TraceAddress => {
                            if trace_address.is_some() {
                                return Err(de::Error::duplicate_field("trace_address"));
                            }
                            trace_address = Some(map.next_value()?); // Vec<usize>
                        },
                        Field::Subtraces => {
                            if subtraces.is_some() {
                                return Err(de::Error::duplicate_field("subtraces"));
                            }
                            subtraces = Some(map.next_value()?); // usize
                        },
                        Field::Action => {
                            if action.is_some() {
                                return Err(de::Error::duplicate_field("action"));
                            }
                            action = Some(map.next_value()?); // serde_json `Value`
                        },
                        Field::Result => {
                            if result.is_some() {
                                return Err(de::Error::duplicate_field("trace_result"));
                            }
                            result = Some(map.next_value()?); // Res
                        },
                        Field::TransactionType => {
                            if tx_type.is_some() {
                                return  Err(de::Error::duplicate_field("transaction_type"));
                            }
                            tx_type = Some(map.next_value()?); // TxType
                        },
                    }
                }

                let tx_type = tx_type.ok_or_else(|| de::Error::missing_field("transaction_type"))?;
                let action = action.ok_or_else(|| de::Error::missing_field("action"))?;
                let action: Action = match tx_type {
                    TxType::Call    => Action::Call(de_value!(action)?),
                    TxType::Create  => Action::Create(de_value!(action)?),
                    TxType::Suicide => Action::Suicide(de_value!(action)?),
                    TxType::Reward  => Action::Reward(de_value!(action)?)
                };
                let result = result.ok_or_else(|| de::Error::missing_field("result"))?;
                let trace_address = trace_address.ok_or_else(|| de::Error::missing_field("trace_address"))?;
                let subtraces = subtraces.ok_or_else(|| de::Error::missing_field("subtraces"))?;

                Ok( TransactionTrace { trace_address, subtraces, action, result } )
            }
        }

        const FIELDS: &'static [&'static str] = &["subtraces", "trace_address", "action", "type", "error", "result" ];
        deserializer.deserialize_struct("Trace", FIELDS, TraceVisitor)
    }
}



// ---------------- VmTrace ------------------------------
#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
/// A record of a full VM trace for a CALL/CREATE.
pub struct VMTrace {
	/// The code to be executed.
	pub code: Bytes,
	/// The operations executed.
	pub ops: Vec<VMOperation>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
/// A record of the execution of a single VM operation.
pub struct VMOperation {
	/// The program counter.
	pub pc: usize,
	/// The gas cost for this instruction.
	pub cost: u64,
	/// Information concerning the execution of the operation.
	pub ex: Option<VMExecutedOperation>,
	/// Subordinate trace of the CALL/CREATE if applicable.
	// #[serde(bound="VMTrace: Deserialize")]
	pub sub: Option<VMTrace>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
/// A record of an executed VM operation.
pub struct VMExecutedOperation {
	/// The total gas used.
	#[serde(rename="used")]
	pub used: u64,
	/// The stack item placed, if any.
	pub push: Vec<U256>,
	/// If altered, the memory delta.
	#[serde(rename="mem")]
	pub mem: Option<MemoryDiff>,
	/// The altered storage value, if any.
	#[serde(rename="store")]
	pub store: Option<StorageDiff>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
/// A diff of some chunk of memory.
pub struct MemoryDiff {
	/// Offset into memory the change begins.
	pub off: usize,
	/// The changed data.
	pub data: Bytes,
}


#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
/// A diff of some storage value.
pub struct StorageDiff {
	/// Which key in storage is changed.
	pub key: U256,
	/// What the value has been changed to.
	pub val: U256,
}



#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // tx: https://etherscan.io/tx/0x550549eb0dde6a4a8f865a19808cb0019e60fac664912e009e5cbae7d5dc0638
    // with 'trace', 'vmTrace', 'stateDiff'
    // with 'trace_call' API function
    const EXAMPLE_TRACE: &'static str = include!("./example-trace-str.rs");

    #[test]
    fn test_serialize_trace_type() {
        let trace_type_str =  r#"["trace","vmTrace","stateDiff"]"#;
        let trace_type = vec![TraceType::Trace, TraceType::VmTrace, TraceType::StateDiff];

        let se_trace_str: String = serde_json::to_string(&trace_type).unwrap();
        assert_eq!(trace_type_str, se_trace_str);
    }

    #[test]
    fn test_deserialize_blocktrace() {
        let _trace: BlockTrace = serde_json::from_str(EXAMPLE_TRACE).unwrap();
    }
}
