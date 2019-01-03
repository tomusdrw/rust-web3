//! Types for the Parity Ad-Hoc Trace API
use types::{H160, H256, U256, Bytes, Action, Res, Address};
use std::collections::BTreeMap;
use serde_derive::{Deserialize, Serialize};

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

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
/// Aux type for Diff::Changed.
pub struct ChangedType<T> {
    from: T,
    to: T,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
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

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
/// Serde-friendly `AccountDiff` shadow.
pub struct AccountDiff {
    pub balance: Diff<U256>,
    pub nonce: Diff<U256>,
    pub code: Diff<Bytes>,
    pub storage: BTreeMap<H256, Diff<H256>>,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
/// Serde-friendly `StateDiff` shadow.
pub struct StateDiff(BTreeMap<H160, AccountDiff>);

// ------------------ Trace -------------
/// Trace
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct TransactionTrace {
	/// Trace address
    #[serde(rename = "traceAddress")]
	trace_address: Vec<Address>,
	/// Subtraces
	subtraces: usize,
	/// Action
	action: Action,
	/// Result
	result: Option<Res>,
}

// ---------------- VmTrace ------------------------------
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
/// A record of a full VM trace for a CALL/CREATE.
pub struct VMTrace {
	/// The code to be executed.
	pub code: Bytes,
	/// The operations executed.
	pub ops: Vec<VMOperation>,
}

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
/// A diff of some chunk of memory.
pub struct MemoryDiff {
	/// Offset into memory the change begins.
	pub off: usize,
	/// The changed data.
	pub data: Bytes,
}


#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
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
