//! Types for getting peer information
use ethereum_types::U256;

use serde_derive::{Deserialize, Serialize};

/// Stores active peer count, connected count, max connected peers
/// and a list of peers for parity node
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ParityPeerType {
    /// number of active peers
    pub active: usize,
    /// number of connected peers
    pub connected: usize,
    /// maximum number of peers that can connect
    pub max: u32,
    /// list of all peers with details
    pub peers: Vec<ParityPeerInfo>,
}

/// details of a peer
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct ParityPeerInfo {
    /// id of peer
    pub id: Option<String>,
    /// name of peer if set by user
    pub name: String,
    /// sync logic for protocol messaging
    pub caps: Vec<String>,
    /// remote address and local address
    pub network: PeerNetworkInfo,
    /// protocol version of peer
    pub protocols: PeerProtocolsInfo,
}

/// ip address of both local and remote
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PeerNetworkInfo {
    /// remote peer address
    pub remote_address: String,
    /// local peer address
    pub local_address: String,
}

/// chain protocol info
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct PeerProtocolsInfo {
    /// chain info
    pub eth: Option<EthProtocolInfo>,
    /// chain info
    pub pip: Option<PipProtocolInfo>,
}

/// eth chain version, difficulty, and head of chain
/// which soft fork? Olympic, Frontier, Homestead, Metropolis, Serenity, etc.
#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct EthProtocolInfo {
    /// version
    pub version: u32,
    /// difficulty
    pub difficulty: Option<U256>,
    /// head of chain
    pub head: String,
}

/// pip version, difficulty, and head
#[derive(Serialize, PartialEq, Clone, Deserialize, Debug)]
pub struct PipProtocolInfo {
    /// version
    pub version: u32,
    /// difficulty
    pub difficulty: U256,
    /// head of chain
    pub head: String,
}
