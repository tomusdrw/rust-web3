//! Types for the Parity Transaction-Trace Filtering API
use types::{BlockNumber, H160, H256, U256, Bytes, Address};
use serde_json::{self, value};
use serde::de::{self, Deserialize, Deserializer, Visitor, MapAccess};
use std::fmt;

/// Trace filter
#[derive(Debug, Default, Clone, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TraceFilter {
    /// From block
    #[serde(rename="fromBlock", skip_serializing_if = "Option::is_none")]
    from_block: Option<BlockNumber>,
    /// To block
    #[serde(rename="toBlock", skip_serializing_if = "Option::is_none")]
    to_block: Option<BlockNumber>,
    /// From address
    #[serde(rename="fromAddress", skip_serializing_if = "Option::is_none")]
    from_address: Option<Vec<Address>>,
    /// To address
    #[serde(rename="toAddress", skip_serializing_if = "Option::is_none")]
    to_address: Option<Vec<Address>>,
    /// Output offset
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<usize>,
    /// Output amount
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
}

/// Trace Filter Builder
#[derive(Default, Debug, Clone, PartialEq)]
pub struct TraceFilterBuilder {
    filter: TraceFilter,
}

impl TraceFilterBuilder {

    /// Sets From block
    pub fn from_block(mut self, block: BlockNumber) -> Self {
        self.filter.from_block = Some(block);
        self
    }

    /// Sets to block
    pub fn to_block(mut self, block: BlockNumber) -> Self {
        self.filter.to_block = Some(block);
        self
    }

    /// Sets to address
    pub fn to_address(mut self, address: Vec<H160>) -> Self {
        self.filter.to_address = Some(address);
        self
    }

    /// Sets from address
    pub fn from_address(mut self, address: Vec<H160>) -> Self {
        self.filter.from_address = Some(address);
        self
    }

    /// Sets after offset
    pub fn after(mut self, after: usize) -> Self {
        self.filter.after = Some(after);
        self
    }

    /// Sets amount of traces to display
    pub fn count(mut self, count: usize) -> Self {
        self.filter.count = Some(count);
        self
    }

    /// Builds the Filter
    pub fn build(&self) -> TraceFilter {
        self.filter.clone()
    }
}

// `LocalizedTrace` in Parity
/// Trace-Filtering API trace type
#[derive(Debug, PartialEq, Clone, Serialize)]
pub struct Trace {
    /// Action
    pub action: Action,
    /// Result
    pub result: Res,
    /// Trace address
    pub trace_address: Vec<usize>,
    /// Subtraces
    pub subtraces: usize,
    /// Transaction position
    pub transaction_position: Option<usize>,
    /// Transaction hash
    pub transaction_hash: Option<H256>,
    /// Block Number
    pub block_number: u64,
    /// Block Hash
    pub block_hash: H256,
}

macro_rules! de_value {
    ($action: ident) => ({
        serde_json::from_value($action).map_err(|e| de::Error::custom(e.to_string()))
    })
}
// a pretty standard custom deserialize, except it deserializes 'error' and 'result' of JSON
// into the result enum, as well as deserializes `Action` based upon `type` field of the JSON.
impl<'de> Deserialize<'de> for Trace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: Deserializer<'de> {
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "lowercase")]
        enum TxType { Call, Create, Suicide, Reward };

        enum Field {
            Action,
            Result,
            TraceAddress,
            Subtraces,
            TransactionPosition,
            TransactionHash,
            BlockNumber,
            BlockHash,
            TxType
        };

        struct TraceVisitor;
        impl<'de> Deserialize<'de> for Field {
            fn deserialize<D>(deserializer: D) -> Result<Field, D::Error>
                where
                    D: Deserializer<'de>,
            {
                struct FieldVisitor;
                // need custom impl, because Result can either be in `result` field or `error` field of JSON
                impl<'de> Visitor<'de> for FieldVisitor {
                    type Value = Field;

                    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                        formatter.write_str("`action`, `result`, `traceAddress`, `subtraces`, \
                                            `transactionPosition`, `transactionHash`, \
                                            `blockNumber`, or `blockHash`")
                    }

                    fn visit_str<E>(self, value: &str) -> Result<Field, E>
                        where
                            E: de::Error,
                    {
                        match value {
                            "action" => Ok(Field::Action),
                            "traceAddress" => Ok(Field::TraceAddress),
                            "subtraces" => Ok(Field::Subtraces),
                            "transactionPosition" => Ok(Field::TransactionPosition),
                            "transactionHash" => Ok(Field::TransactionHash),
                            "blockNumber" => Ok(Field::BlockNumber),
                            "blockHash" => Ok(Field::BlockHash),
                            "error" => Ok(Field::Result),
                            "result" => Ok(Field::Result),
                            "type" => Ok(Field::TxType),
                            _ => Err(de::Error::unknown_field(value, FIELDS))
                        }
                    }
                }
                deserializer.deserialize_identifier(FieldVisitor)
            }
        }

        impl<'de> Visitor<'de> for TraceVisitor {
            type Value = Trace;
            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("Trace-Filtering Trace struct")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
                where
                    M: MapAccess<'de>
            {
                let mut action: Option<value::Value> = None;
                let mut result = None;
                let mut trace_address = None;
                let mut subtraces = None;
                let mut transaction_position = None;
                let mut transaction_hash = None;
                let mut block_number = None;
                let mut block_hash = None;
                let mut tx_type = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Action => {
                            if action.is_some() {
                                return Err(de::Error::duplicate_field("action"));
                            }
                            action = Some(map.next_value()?); // serde_json `Value`
                        },
                        Field::Result => {
                            if result.is_some() {
                                return Err(de::Error::duplicate_field("result"));
                            }
                            result = Some(map.next_value()?); // Res
                        },
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
                        Field::TransactionPosition => {
                            if transaction_position.is_some() {
                                return Err(de::Error::duplicate_field("transaction_position"));
                            }
                            transaction_position = Some(map.next_value()?); // usize
                        },
                        Field::TransactionHash => {
                            if transaction_hash.is_some() {
                                return Err(de::Error::duplicate_field("transaction_hash"));
                            }
                            transaction_hash = Some(map.next_value()?); // H256
                        },
                        Field::BlockNumber => {
                            if block_number.is_some() {
                                return Err(de::Error::duplicate_field("block_number"));
                            }
                            block_number = Some(map.next_value()?); // u64
                        },
                        Field::BlockHash => {
                            if block_hash.is_some() {
                                return Err(de::Error::duplicate_field("block_hash"));
                            }
                            block_hash = Some(map.next_value()?); // H256
                        },
                        Field::TxType => {
                            if tx_type.is_some() {
                                return Err(de::Error::duplicate_field("type"));
                            }
                            tx_type = Some(map.next_value()?); // TxType
                        }
                    }
                }

                // check to make sure TxType + action was deserialized
                let tx_type = tx_type.ok_or_else(|| de::Error::missing_field("tx_type"))?;
                let action = action.ok_or_else(|| de::Error::missing_field("action"))?;
                // deserialize correct action struct variant type from TxType
                let action = match tx_type {
                    TxType::Call    => Action::Call(de_value!(action)?),
                    TxType::Create  => Action::Create(de_value!(action)?),
                    TxType::Suicide => Action::Suicide(de_value!(action)?),
                    TxType::Reward  => Action::Reward(de_value!(action)?),
                };
                // make sure of the rest of the fields
                let result               = result.ok_or_else(|| de::Error::missing_field("result"))?;
                let trace_address        = trace_address.ok_or_else(|| de::Error::missing_field("trace_address"))?;
                let subtraces            = subtraces.ok_or_else(|| de::Error::missing_field("subtraces"))?;
                let transaction_position = transaction_position.ok_or_else(|| de::Error::missing_field("transaction_position"))?;
                let transaction_hash     = transaction_hash.ok_or_else(|| de::Error::missing_field("transaction_hash"))?;
                let block_number         = block_number.ok_or_else(|| de::Error::missing_field("block_number"))?;
                let block_hash           = block_hash.ok_or_else(|| de::Error::missing_field("block_hash"))?;
                Ok(Trace {action, result, trace_address, subtraces, transaction_position, transaction_hash, block_number, block_hash } )
            }

        }

        const FIELDS: &'static [&'static str] = &["action", "result", "error", "traceAddress", "subtraces",
            "transactionPosition", "transactionHash", "blockNumber", "blockHash"];

        deserializer.deserialize_struct("Trace", FIELDS, TraceVisitor)
    }
}

/// Response
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Res {
    /// Call
    Call(CallResult),
    /// Create
    Create(CreateResult),
    /// Call or Create failure
    FailedCallOrCreate(String),
    /// None
    None,
}

impl Default for Res {
    fn default() -> Res {
        Res::None
    }
}

/// Action
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub enum Action {
    /// Call
    Call(Call),
    /// Create
    Create(Create),
    /// Suicide
    Suicide(Suicide),
    /// Reward
    Reward(Reward),
}

/// Call Result
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct CallResult {
    /// Gas used
    #[serde(rename="gasUsed")]
    pub gas_used: U256,
    /// Output bytes
    pub output: Bytes,
}

/// Craete Result
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct CreateResult {
    /// Gas used
    #[serde(rename="gasUsed")]
    pub gas_used: U256,
    /// Code
    pub code: Bytes,
    /// Assigned address
    pub address: Address,
}

/// Call response
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Call {
    /// Sender
    pub from: Address,
    /// Recipient
    pub to: Address,
    /// Transfered Value
    pub value: U256,
    /// Gas
    pub gas: U256,
    /// Input data
    pub input: Bytes,
    /// The type of the call.
    #[serde(rename="callType")]
    pub call_type: CallType,
}

/// Call type.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum CallType {
    /// None
    #[serde(rename="none")]
    None,
    /// Call
    #[serde(rename="call")]
    Call,
    /// Call code
    #[serde(rename="callcode")]
    CallCode,
    /// Delegate call
    #[serde(rename="delegatecall")]
    DelegateCall,
    /// Static call
    #[serde(rename="staticcall")]
    StaticCall,
}

impl Default for CallType {
    fn default() -> CallType {
        CallType::None
    }
}

/// Create response
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Create {
    /// Sender
    pub from: Address,
    /// Value
    pub value: U256,
    /// Gas
    pub gas: U256,
    /// Initialization code
    pub init: Bytes,
}

/// Suicide
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct Suicide {
    /// Address.
    pub address: Address,
    /// Refund address.
    #[serde(rename="refundAddress")]
    pub refund_address: Address,
    /// Balance.
    pub balance: U256,
}

/// Reward action
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct Reward {
    /// Author's address.
    pub author: Address,
    /// Reward amount.
    pub value: U256,
    /// Reward type.
    #[serde(rename="rewardType")]
    pub reward_type: RewardType,
}

/// Reward type.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum RewardType {
    /// Block
    #[serde(rename="block")]
    Block,
    /// Uncle
    #[serde(rename="uncle")]
    Uncle,
    /// EmptyStep (AuthorityRound)
    #[serde(rename="emptyStep")]
    EmptyStep,
    /// External (attributed as part of an external protocol)
    #[serde(rename="external")]
    External,
}



#[cfg(test)]
mod tests {
    use super::*;


    const EXAMPLE_TRACE: &'static str =
    r#"{
        "action": {
            "callType": "call",
            "from": "0xd1220a0cf47c7b9be7a2e6ba89f429762e7b9adb",
            "gas": "0x63ab9",
            "input": "0xb9f256cd000000000000000000000000fb6916095ca1df60bb79ce92ce3ea74c37c5d3590000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000008000000000000000000000000000000000000000000000000000000000000001a000000000000000000000000000000000000000000000000000000000000000e85468697320697320746865206f6666696369616c20457468657265756d20466f756e646174696f6e20546970204a61722e20466f722065766572792061626f76652061206365727461696e2076616c756520646f6e6174696f6e207765276c6c2063726561746520616e642073656e6420746f20796f752061206272616e64206e657720556e69636f726e20546f6b656e2028f09fa684292e20436865636b2074686520756e69636f726e2070726963652062656c6f77202831206574686572203d20313030302066696e6e6579292e205468616e6b7320666f722074686520737570706f72742100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
            "to": "0xfb6916095ca1df60bb79ce92ce3ea74c37c5d359",
            "value": "0x0"
        },
        "blockHash": "0x6474a53a9ebf72d306a1406ec12ded12e210b6c3141b4373bfb3a3cea987dfb8",
        "blockNumber": 988775,
        "result": {
            "gasUsed": "0x4b419",
            "output": "0x0000000000000000000000000000000000000000000000000000000000000000"
        },
        "subtraces": 1,
        "traceAddress": [],
        "transactionHash": "0x342c284238149db221f9d87db87f90ffad7ac0aac57c0c480142f4c21b63f652",
        "transactionPosition": 1,
        "type": "call"
    }"#;

    #[test]
    fn test_deserialize_trace() {
        let _trace: Trace = serde_json::from_str(EXAMPLE_TRACE).unwrap();
    }
}

