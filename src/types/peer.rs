//! Types for getting peer information
use ethereum_types::U256;
use serde_derive::{Deserialize, Serialize};

/// Stores active peer count, connected count, max connected peers
/// and a list of peers
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerType {
    /// number of active peers
    pub active: usize,
    /// number of connected peers
    pub connected: usize,
    /// maximum number of peers that can connect
    pub max: u32,
    /// list of all peers with details
    pub peers: Vec<PeerInfo>,
}

/// details of a peer
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerInfo {
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
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all="camelCase")]
pub struct PeerNetworkInfo {
    pub remote_address: String,
    pub local_address: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerProtocolsInfo {
    pub eth: Option<EthProtocolInfo>,
    pub pip: Option<PipProtocolInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EthProtocolInfo {
    pub version: u32,
    pub difficulty: Option<U256>,
    pub head: String,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct PipProtocolInfo {
    pub version: u32,
    pub difficulty: U256,
    pub head: String,
}

