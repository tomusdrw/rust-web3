use types::{Bytes, H160, H256, Index, Log, U256, U64};

/// Description of a Transaction, pending or in the chain.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct Transaction {
    /// Hash
    pub hash: H256,
    /// Nonce
    pub nonce: U256,
    /// Block hash. None when pending.
    #[serde(rename = "blockHash")]
    pub block_hash: Option<H256>,
    /// Block number. None when pending.
    #[serde(rename = "blockNumber")]
    pub block_number: Option<U256>,
    /// Transaction Index. None when pending.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<Index>,
    /// Sender
    pub from: H160,
    /// Recipient (None when contract creation)
    pub to: Option<H160>,
    /// Transfered value
    pub value: U256,
    /// Gas Price
    #[serde(rename = "gasPrice")]
    pub gas_price: U256,
    /// Gas amount
    pub gas: U256,
    /// Input data
    pub input: Bytes,
}

/// "Receipt" of an executed transaction: details of its execution.
#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct Receipt {
    /// Transaction hash.
    #[serde(rename = "transactionHash")]
    pub transaction_hash: H256,
    /// Index within the block.
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Index,
    /// Hash of the block this transaction was included within.
    #[serde(rename = "blockHash")]
    pub block_hash: H256,
    /// Number of the block this transaction was included within.
    #[serde(rename = "blockNumber")]
    pub block_number: U256,
    /// Cumulative gas used within the block after this was executed.
    #[serde(rename = "cumulativeGasUsed")]
    pub cumulative_gas_used: U256,
    /// Gas used by this transaction alone.
    #[serde(rename = "gasUsed")]
    pub gas_used: U256,
    /// Contract address created, or `None` if not a deployment.
    #[serde(rename = "contractAddress")]
    pub contract_address: Option<H160>,
    /// Logs generated within this transaction.
    pub logs: Vec<Log>,
    /// Status: either 1 (success) or 0 (failure).
    pub status: Option<U64>,
}

#[cfg(test)]
mod tests {
    use serde_json;
    use super::Receipt;

    #[test]
    fn test_deserialize_receipt() {
        let receipt_str = "{\"blockHash\":\"0x83eaba432089a0bfe99e9fc9022d1cfcb78f95f407821be81737c84ae0b439c5\",\"blockNumber\":\"0x38\",\"contractAddress\":\"0x03d8c4566478a6e1bf75650248accce16a98509f\",\"cumulativeGasUsed\":\"0x927c0\",\"gasUsed\":\"0x927c0\",\"logs\":[],\"logsBloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"root\":null,\"transactionHash\":\"0x422fb0d5953c0c48cbb42fb58e1c30f5e150441c68374d70ca7d4f191fd56f26\",\"transactionIndex\":\"0x0\"}";

        let _receipt: Receipt = serde_json::from_str(receipt_str).unwrap();
    }

    #[test]
    fn should_deserialize_receipt_with_status() {
        let receipt_str = r#"{
        "blockHash": "0x83eaba432089a0bfe99e9fc9022d1cfcb78f95f407821be81737c84ae0b439c5",
        "blockNumber": "0x38",
        "contractAddress": "0x03d8c4566478a6e1bf75650248accce16a98509f",
        "cumulativeGasUsed": "0x927c0",
        "gasUsed": "0x927c0",
        "logs": [],
        "logsBloom": "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000",
        "root": null,
        "transactionHash": "0x422fb0d5953c0c48cbb42fb58e1c30f5e150441c68374d70ca7d4f191fd56f26",
        "transactionIndex": "0x0",
        "status": "0x1"
    }"#;

        let _receipt: Receipt = serde_json::from_str(receipt_str).unwrap();
    }
}
