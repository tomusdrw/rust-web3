use crate::types::U256;
use serde::de::{Deserializer, Error};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};

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

// Sync info from subscription has a different key format
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SubscriptionSyncInfo {
    /// The block at which import began.
    pub starting_block: U256,

    /// The highest currently synced block.
    pub current_block: U256,

    /// The estimated highest block.
    pub highest_block: U256,
}

impl From<SubscriptionSyncInfo> for SyncInfo {
    fn from(s: SubscriptionSyncInfo) -> Self {
        Self {
            starting_block: s.starting_block,
            current_block: s.current_block,
            highest_block: s.highest_block,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SubscriptionSyncState {
    pub syncing: bool,
    pub status: Option<SubscriptionSyncInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum SyncStateVariants {
    Rpc(SyncInfo),
    Subscription(SubscriptionSyncState),
    Boolean(bool),
}

// The `eth_syncing` method returns either `false` or an instance of the sync info object.
// This doesn't play particularly well with the features exposed by `serde_derive`,
// so we use the custom impls below to ensure proper behavior.
impl<'de> Deserialize<'de> for SyncState {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let v: SyncStateVariants = Deserialize::deserialize(deserializer)?;
        match v {
            SyncStateVariants::Rpc(info) => Ok(SyncState::Syncing(info)),
            SyncStateVariants::Subscription(state) => match state.status {
                None if !state.syncing => Ok(SyncState::NotSyncing),
                Some(ref info) if state.syncing => Ok(SyncState::Syncing(info.clone().into())),
                _ => Err(D::Error::custom(
                    "expected object or `syncing = false`, got `syncing = true`",
                )),
            },
            SyncStateVariants::Boolean(boolean) => {
                if !boolean {
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

#[cfg(test)]
mod tests {
    use super::{SyncInfo, SyncState};

    use serde_json;

    #[test]
    fn should_deserialize_rpc_sync_info() {
        let sync_state = r#"{
          "currentBlock": "0x42",
          "highestBlock": "0x9001",
          "knownStates": "0x1337",
          "pulledStates": "0x13",
          "startingBlock": "0x0"
        }"#;

        let value: SyncState = serde_json::from_str(sync_state).unwrap();

        assert_eq!(
            value,
            SyncState::Syncing(SyncInfo {
                starting_block: 0x0.into(),
                current_block: 0x42.into(),
                highest_block: 0x9001.into()
            })
        );
    }

    #[test]
    fn should_deserialize_subscription_sync_info() {
        let sync_state = r#"{
          "syncing": true,
          "status": {
            "CurrentBlock": "0x42",
            "HighestBlock": "0x9001",
            "KnownStates": "0x1337",
            "PulledStates": "0x13",
            "StartingBlock": "0x0"
          }
        }"#;

        let value: SyncState = serde_json::from_str(sync_state).unwrap();

        assert_eq!(
            value,
            SyncState::Syncing(SyncInfo {
                starting_block: 0x0.into(),
                current_block: 0x42.into(),
                highest_block: 0x9001.into()
            })
        );
    }

    #[test]
    fn should_deserialize_boolean_not_syncing() {
        let sync_state = r#"false"#;
        let value: SyncState = serde_json::from_str(sync_state).unwrap();

        assert_eq!(value, SyncState::NotSyncing);
    }

    #[test]
    fn should_deserialize_subscription_not_syncing() {
        let sync_state = r#"{
          "syncing": false
        }"#;

        let value: SyncState = serde_json::from_str(sync_state).unwrap();

        assert_eq!(value, SyncState::NotSyncing);
    }

    #[test]
    fn should_not_deserialize_invalid_boolean_syncing() {
        let sync_state = r#"true"#;
        let res: Result<SyncState, _> = serde_json::from_str(sync_state);
        assert!(res.is_err());
    }

    #[test]
    fn should_not_deserialize_invalid_subscription_syncing() {
        let sync_state = r#"{
          "syncing": true
        }"#;

        let res: Result<SyncState, _> = serde_json::from_str(sync_state);
        assert!(res.is_err());
    }

    #[test]
    fn should_not_deserialize_invalid_subscription_not_syncing() {
        let sync_state = r#"{
          "syncing": false,
          "status": {
            "CurrentBlock": "0x42",
            "HighestBlock": "0x9001",
            "KnownStates": "0x1337",
            "PulledStates": "0x13",
            "StartingBlock": "0x0"
          }
        }"#;

        let res: Result<SyncState, _> = serde_json::from_str(sync_state);
        assert!(res.is_err());
    }
}
