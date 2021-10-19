use crate::types::{BlockNumber, U256};
use serde::{Deserialize, Serialize};

/// The fee history type returned from RPC calls.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeHistory {
    /// Oldest block
    pub oldest_block: BlockNumber,
    /// Base fee per gas
    pub base_fee_per_gas: Vec<U256>,
    /// Gas used ratio
    pub gas_used_ratio: Vec<f64>,
    /// Reward
    pub reward: Option<Vec<Vec<U256>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_history() {
        let fee_history = FeeHistory {
            oldest_block: BlockNumber::Number(123456.into()),
            base_fee_per_gas: vec![100.into(), 110.into()],
            gas_used_ratio: vec![1.0, 2.0, 3.0],
            reward: None,
        };

        let serialized = serde_json::to_value(fee_history.clone()).unwrap();
        let deserialized = serde_json::from_value(serialized).unwrap();

        assert_eq!(fee_history, deserialized);
    }
}
