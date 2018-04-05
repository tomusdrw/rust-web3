use serde::de::{Deserialize, DeserializeOwned, Deserializer, Error};
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

// `eth_subscribe(syncing)` returns a SyncInfo object with a different format
fn deserialize_subscription<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    use serde_json::Value;
    use std::collections::BTreeMap;

    let map = BTreeMap::<String, Value>::deserialize(deserializer)?;
    let renamed = map.into_iter()
        .map(|(k, v)| {
            let mut cs = k.chars();
            let nk = match cs.next() {
                None => String::new(),
                Some(c) => c.to_lowercase().collect::<String>() + cs.as_str(),
            };
            let nv = Value::String(format!("0x{:x}", v.as_u64().unwrap_or(0u64)));

            (nk, nv)
        })
        .collect();
    T::deserialize(Value::Object(renamed)).map_err(Error::custom)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct SubscriptionSyncInfo {
    pub syncing: bool,

    #[serde(deserialize_with = "deserialize_subscription")]
    pub status: Option<SyncInfo>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
enum SyncStateVariants {
    Subscription(SubscriptionSyncInfo),
    Rpc(SyncInfo),
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
            SyncStateVariants::Subscription(info) => {
                if !info.syncing {
                    Ok(SyncState::NotSyncing)
                } else if let Some(status) = info.status {
                    Ok(SyncState::Syncing(status))
                } else {
                    Err(D::Error::custom("syncing is `true` but no status reported"))
                }
            }
            SyncStateVariants::Rpc(info) => Ok(SyncState::Syncing(info)),
            SyncStateVariants::Boolean(boolian) => {
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
