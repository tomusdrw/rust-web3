use ethereum_types::U256;
use serde_derive::{Deserialize, Serialize};

use std::fmt;

#[derive(Serialize, Deserialize, Clone)]
pub struct PeerType {
    pub active: usize,
    pub connected: usize,
    pub max: u32,
    pub peers: Vec<PeerInfo>,
}

impl fmt::Debug for PeerType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "active: {}, connected: {}, max: {}, peers: {:?}", self.active, self.connected, self.max, self.peers)

    }
}


#[derive(Serialize, Deserialize, Clone)]
pub struct PeerInfo {
    pub id: Option<String>,
    pub name: String,
    pub caps: Vec<String>,
    pub network: PeerNetworkInfo,
    pub protocols: PeerProtocolsInfo,
}
impl fmt::Debug for PeerInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id: {:?}, name: {}, caps: {:?}, remote address: {}, local address: {}", self.id, self.name, self.caps, self.network.remote_address, self.network.local_address)
    }
}


#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct PeerNetworkInfo {

    #[serde(rename="remoteAddress")]
    pub remote_address: String,

    #[serde(rename="localAddress")]
    pub local_address: String,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
pub struct PeerProtocolsInfo {
    pub eth: Option<EthProtocolInfo>,
    pub pip: Option<PipProtocolInfo>,
}

#[derive(Serialize, Clone, Deserialize, Debug)]
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

