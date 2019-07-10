use crate::types::{H256, U256};
use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{self, Value};

/// Miner's work package
#[derive(Debug, PartialEq, Eq)]
pub struct Work {
    /// The proof-of-work hash.
    pub pow_hash: H256,
    /// The seed hash.
    pub seed_hash: H256,
    /// The target.
    pub target: H256,
    /// The block number: this isn't always stored.
    pub number: Option<u64>,
}

impl<'a> Deserialize<'a> for Work {
    fn deserialize<D>(deserializer: D) -> Result<Work, D::Error>
    where
        D: Deserializer<'a>,
    {
        let v: Value = Deserialize::deserialize(deserializer)?;

        let (pow_hash, seed_hash, target, number) = serde_json::from_value::<(H256, H256, H256, u64)>(v.clone())
            .map(|(pow_hash, seed_hash, target, number)| (pow_hash, seed_hash, target, Some(number)))
            .or_else(|_| {
                serde_json::from_value::<(H256, H256, H256)>(v)
                    .map(|(pow_hash, seed_hash, target)| (pow_hash, seed_hash, target, None))
            })
            .map_err(|e| D::Error::custom(format!("Cannot deserialize Work: {:?}", e)))?;

        Ok(Work {
            pow_hash,
            seed_hash,
            target,
            number,
        })
    }
}

impl Serialize for Work {
    fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self.number.as_ref() {
            Some(num) => (&self.pow_hash, &self.seed_hash, &self.target, U256::from(*num)).serialize(s),
            None => (&self.pow_hash, &self.seed_hash, &self.target).serialize(s),
        }
    }
}
