#[cfg(feature = "account-zero")]
use serde::Deserializer;
use types::{Bytes, Index, Log, H160, H256, U256, U64};

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
    #[cfg_attr(feature = "account-zero", serde(deserialize_with = "deserialize_to"))]
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

#[cfg(feature = "account-zero")]
fn deserialize_to<'de, D>(deserializer: D) -> Result<Option<H160>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::{de, export::fmt};
    use serde_json;

    struct Visitor;

    impl<'de> de::Visitor<'de> for Visitor {
        type Value = Option<H160>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("0x-prefixed hex string")
        }

        fn visit_str<E>(self, value: &str) -> Result<Option<H160>, E>
        where
            E: de::Error,
        {
            match value {
                "0x0" => Ok(None),
                value => {
                    let result = serde_json::from_str(&format!("{:?}", value)).map_err(E::custom)?;
                    Ok(Some(result))
                }
            }
        }
    }
    deserializer.deserialize_str(Visitor)
}

#[cfg(test)]
mod tests {
    use super::Receipt;
    use super::Transaction;
    use serde_json;

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

    #[test]
    fn should_deserialize_transaction() {
        let transaction_str = r#"{
                "hash": "0x53b9c569584541f8bd1c874ea6409acc45fefb249af8e58b26936b54a0da140b",
                "nonce": "0x0",
                "blockHash": "0x0ae7d24c16f6e8cd16a732bc72e8b367e775e2be7cd1af128c58478038d28388",
                "blockNumber": "0x02",
                "transactionIndex": "0x00",
                "from": "0x96984c3e77f38ed01d1c3d98f4bd7c8b11d51d7e",

"to": "0xa00f2cac7bad9285ecfd59e8860f5b2d8622e099",

                "value": "0x0",
                "gas": "0x100000",
                "gasPrice": "0x01",
                "input": "0xf5c4e30096e501347bcec8adb459eb0b8703af22dbda7382a04fea75110fa812"
            }"#;

        let _transaction: Transaction = serde_json::from_str(transaction_str).unwrap();
    }

    #[cfg(feature = "account-zero")]
    #[test]
    fn should_deserialize_transaction_with_to_address_zero() {
        let transaction_str = r#"{
                "hash": "0xecee5907db9bb4b2e8c48fded723174e386b74a8fc8b74f17e6cace1655caefe",
                "nonce": "0x0",
                "blockHash": "0xf04cac6878b58993abe986c15acf00f07887b9baf9bfd6d5b5097d52455c4ee5",
                "blockNumber": "0x01",
                "transactionIndex": "0x00",
                "from": "0x147ba99ef89c152f8004e91999fee87bda6cbc3e",
                "to": "0x0",
                "value": "0x8ac7230489e80000",
                "gas": "0x0170c0",
                "gasPrice": "0x77359400",
                "input": "0x426000526063601b53610082600561001b01602039610082601bf36350000005602060006000376020602160206000600060026048f17f49195bfd5b5b53f1a30135c8eabffbefa39be1dde06aa43736b0c5ca570654d560215114166054574203630000a8c010606b5760006000f35b733853005576eeca2ba7ec579ce04c1ce2f4cd162eff5b7350358ea110e2ae0e509cd0dee177064c58e4a1e5ff"
            }"#;

        let _transaction: Transaction = serde_json::from_str(transaction_str).unwrap();
    }
}
