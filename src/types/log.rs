use types::{BlockNumber, Bytes, H160, H256, U256};

/// A log produced by a transaction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Log {
    /// H160
    pub address: H160,
    /// Topics
    pub topics: Vec<H256>,
    /// Data
    pub data: Bytes,
    /// Block Hash
    #[serde(rename = "blockHash")]
    pub block_hash: Option<H256>,
    /// Block Number
    #[serde(rename = "blockNumber")]
    pub block_number: Option<U256>,
    /// Transaction Hash
    #[serde(rename = "transactionHash")]
    pub transaction_hash: Option<H256>,
    /// Transaction Index
    #[serde(rename = "transactionIndex")]
    pub transaction_index: Option<U256>,
    /// Log Index in Block
    #[serde(rename = "logIndex")]
    pub log_index: Option<U256>,
    /// Log Index in Transaction
    #[serde(rename = "transactionLogIndex")]
    pub transaction_log_index: Option<U256>,
    /// Log Type
    #[serde(rename = "logType")]
    pub log_type: Option<String>,
    /// Removed
    pub removed: Option<bool>,
}

impl Log {
    /// Returns true if the log has been removed.
    pub fn is_removed(&self) -> bool {
        match self.removed {
            Some(val_removed) => return val_removed,
            None => (),
        }
        match self.log_type {
            Some(ref val_log_type) => {
                if val_log_type == "removed" {
                    return true;
                }
            }
            None => (),
        }
        return false;
    }
}

/// Filter
#[derive(Default, Debug, PartialEq, Clone, Serialize)]
pub struct Filter {
    /// From Block
    #[serde(rename = "fromBlock")]
    from_block: Option<BlockNumber>,
    /// To Block
    #[serde(rename = "toBlock")]
    to_block: Option<BlockNumber>,
    /// Address
    address: Option<Vec<H160>>,
    /// Topics
    topics: Option<Vec<Option<Vec<H256>>>>,
    /// Limit
    limit: Option<usize>,
}

/// Filter Builder
#[derive(Default, Clone)]
pub struct FilterBuilder {
    filter: Filter,
}

impl FilterBuilder {
    /// Sets from block
    pub fn from_block(mut self, block: BlockNumber) -> Self {
        self.filter.from_block = Some(block);
        self
    }

    /// Sets to block
    pub fn to_block(mut self, block: BlockNumber) -> Self {
        self.filter.to_block = Some(block);
        self
    }

    /// Single address
    pub fn address(mut self, address: Vec<H160>) -> Self {
        self.filter.address = Some(address);
        self
    }

    /// Topics
    pub fn topics(mut self, topic1: Option<Vec<H256>>, topic2: Option<Vec<H256>>, topic3: Option<Vec<H256>>, topic4: Option<Vec<H256>>) -> Self {
        self.filter.topics = Some(vec![topic1, topic2, topic3, topic4]);
        self
    }

    /// Limit the result
    pub fn limit(mut self, limit: usize) -> Self {
        self.filter.limit = Some(limit);
        self
    }

    /// Returns filter
    pub fn build(&self) -> Filter {
        self.filter.clone()
    }
}

#[cfg(test)]
mod tests {
    use types::log::{Bytes, Log};

    #[test]
    fn is_removed_removed_true() {
        let log = Log {
            address: 1.into(),
            topics: vec![],
            data: Bytes(vec![]),
            block_hash: Some(2.into()),
            block_number: Some(1.into()),
            transaction_hash: Some(3.into()),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: None,
            removed: Some(true),
        };
        assert_eq!(true, log.is_removed());
    }

    #[test]
    fn is_removed_removed_false() {
        let log = Log {
            address: 1.into(),
            topics: vec![],
            data: Bytes(vec![]),
            block_hash: Some(2.into()),
            block_number: Some(1.into()),
            transaction_hash: Some(3.into()),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: None,
            removed: Some(false),
        };
        assert_eq!(false, log.is_removed());
    }

    #[test]
    fn is_removed_log_type_removed() {
        let log = Log {
            address: 1.into(),
            topics: vec![],
            data: Bytes(vec![]),
            block_hash: Some(2.into()),
            block_number: Some(1.into()),
            transaction_hash: Some(3.into()),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: Some("removed".into()),
            removed: None,
        };
        assert_eq!(true, log.is_removed());
    }

    #[test]
    fn is_removed_log_type_mined() {
        let log = Log {
            address: 1.into(),
            topics: vec![],
            data: Bytes(vec![]),
            block_hash: Some(2.into()),
            block_number: Some(1.into()),
            transaction_hash: Some(3.into()),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: Some("mined".into()),
            removed: None,
        };
        assert_eq!(false, log.is_removed());
    }

    #[test]
    fn is_removed_log_type_and_removed_none() {
        let log = Log {
            address: 1.into(),
            topics: vec![],
            data: Bytes(vec![]),
            block_hash: Some(2.into()),
            block_number: Some(1.into()),
            transaction_hash: Some(3.into()),
            transaction_index: Some(0.into()),
            log_index: Some(0.into()),
            transaction_log_index: Some(0.into()),
            log_type: None,
            removed: None,
        };
        assert_eq!(false, log.is_removed());
    }
}
