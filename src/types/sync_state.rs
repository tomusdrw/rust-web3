use serde::de::{Deserialize, Deserializer, Error};
use serde::ser::{Serialize, Serializer};
use types::U256;

/// Information about current blockchain syncing operations.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncInfo {
    /// The block at which import began.
    pub starting_block: U256,

    /// The highest currently synced block.
    pub current_block: U256,

    /// The estimated highest block.
    pub highest_block: U256,
}

/// The current state of blockchain syncing operations.
#[derive(Debug, Clone, PartialEq)]
pub enum SyncState {
    /// Blockchain is syncing.
    Syncing(SyncInfo),

    /// Blockchain is not syncing.
    NotSyncing,
}

// The `eth_syncing` method returns either `false` or an instance of the sync info object.
// This doesn't play particularly well with the features exposed by `serde_derive`,
// so we use the custom impls below to ensure proper behavior.

impl<'de> Deserialize<'de> for SyncState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let either: Either<SyncInfo, bool> = Deserialize::deserialize(deserializer)?;
        match either {
            Either::A(info) => Ok(SyncState::Syncing(info)),
            Either::B(boolian) => {
                if !boolian {
                    Ok(SyncState::NotSyncing)
                } else {
                    Err(D::Error::custom("expected object or `false`, got `true`"))
                }
            }
        }
    }
}

impl Serialize for SyncState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match *self {
            SyncState::Syncing(ref info) => info.serialize(serializer),
            SyncState::NotSyncing => false.serialize(serializer),
        }
    }
}

/// implementation detail for sync state deserialization
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum Either<A, B> {
    A(A),
    B(B),
}
