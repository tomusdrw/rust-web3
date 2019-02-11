use api::Namespace;
use helpers::{self, CallFuture};
use types::{H256, Address};

use Transport;

#[derive(Debug, Clone)]
/// `Parity_Set` Specific API
pub struct ParitySet<T> {
    transport: T
}

impl<T: Transport> Namespace<T> for ParitySet<T> {
    fn new(transport: T) -> Self
        where
        Self: Sized,
    {
        ParitySet { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> ParitySet<T> {
    
    /// Set Parity to accept non-reserved peers (default behavior)
    pub fn parity_accept_non_reserved_peers(&self) -> CallFuture<bool, T::Out> {
        CallFuture::new(self.transport().execute("acceptNonReservedPeers", vec![]))
    }

    /// Add a reserved peer
    pub fn parity_add_reserved_peer(&self, enode: &str) -> CallFuture<bool, T::Out> {
        let enode = helpers::serialize(&enode);
        CallFuture::new(self.transport().execute("addReservedPeer", vec![enode]))
    }

    /// Set Parity to drop all non-reserved peers. To restore default behavior call parity_acceptNonReservedPeers
    pub fn parity_drop_non_reserved_peers(&self) -> CallFuture<bool, T::Out> {
        CallFuture::new(self.transport().execute("dropNonReservedPeers", vec![]))
    }

    /// Attempts to upgrade Parity to the version specified in parity_upgradeReady
    pub fn parity_execute_upgrade(&self) -> CallFuture<bool, T::Out> {
        CallFuture::new(self.transport().execute("executeUpgrade", vec![]))
    }

    /// Creates a hash of a file at a given URL
    pub fn parity_hash_content(&self, url: &str) -> CallFuture<H256, T::Out> {
        let url = helpers::serialize(&url);
        CallFuture::new(self.transport().execute("hashContent", vec![url]))
    }

    /// Remove a reserved peer
    pub fn parity_remove_reserved_peer(&self, enode: &str) -> CallFuture<bool, T::Out> {
        let enode = helpers::serialize(&enode);
        CallFuture::new(self.transport().execute("removeReservedPeer", vec![enode]))
    }

    /// Changes author (coinbase) for mined blocks
    pub fn parity_set_author(&self, author: &Address) -> CallFuture<bool, T::Out> {
        let address = helpers::serialize(&author);
        CallFuture::new(self.transport().execute("setAuthor", vec![address]))
    }

    /// Sets the network spec file Parity is using
    pub fn parity_set_chain(&self, chain: &str) -> CallFuture<bool, T::Out> {
        let chain = helpers::serialize(&chain);
        CallFuture::new(self.transport().execute("setChain", vec![chain]))
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;

    use api::Namespace;
    use rpc::Value;
    use types::{H256, Address};

    use super::ParitySet;

    rpc_test! (
        ParitySet:parity_accept_non_reserved_peers => "acceptNonReservedPeers";
        Value::Bool(true) => true
    );

    rpc_test! (
        ParitySet:parity_add_reserved_peer,
        "enode://a979fb575495b8d6db44f750317d0f4622bf4c2aa3365d6af7c284339968eef29b69ad0dce72a4d8db5ebb4968de0e3bec910127f134779fbcb0cb6d3331163c@22.99.55.44:7770" 
        => "addReservedPeer", vec![r#""enode://a979fb575495b8d6db44f750317d0f4622bf4c2aa3365d6af7c284339968eef29b69ad0dce72a4d8db5ebb4968de0e3bec910127f134779fbcb0cb6d3331163c@22.99.55.44:7770""#];
        Value::Bool(true) => true
    );

    rpc_test! (
        ParitySet:parity_drop_non_reserved_peers => "dropNonReservedPeers";
        Value::Bool(true) => true
    );

    rpc_test! (
        ParitySet:parity_execute_upgrade => "executeUpgrade";
        Value::Bool(true) => true
    );

    rpc_test! (
        ParitySet:parity_hash_content,
        "https://raw.githubusercontent.com/paritytech/parity-ethereum/master/README.md" 
        => "hashContent", vec![r#""https://raw.githubusercontent.com/paritytech/parity-ethereum/master/README.md""#];
        Value::String("0x5198e0fc1a9b90078c2e5bfbc6ab6595c470622d3c28f305d3433c300bba5a46".into()) => H256::from("0x5198e0fc1a9b90078c2e5bfbc6ab6595c470622d3c28f305d3433c300bba5a46")
    );

    rpc_test! (
        ParitySet:parity_remove_reserved_peer,
        "enode://a979fb575495b8d6db44f750317d0f4622bf4c2aa3365d6af7c284339968eef29b69ad0dce72a4d8db5ebb4968de0e3bec910127f134779fbcb0cb6d3331163c@22.99.55.44:7770"
        => "removeReservedPeer", vec![r#""enode://a979fb575495b8d6db44f750317d0f4622bf4c2aa3365d6af7c284339968eef29b69ad0dce72a4d8db5ebb4968de0e3bec910127f134779fbcb0cb6d3331163c@22.99.55.44:7770""#];
        Value::Bool(true) => true
    );
    
    rpc_test! (
        ParitySet:parity_set_author, &Address::from("0x407d73d8a49eeb85d32cf465507dd71d507100c1")
        => "setAuthor", vec![r#""0x407d73d8a49eeb85d32cf465507dd71d507100c1""#];
        Value::Bool(true) => true
    );

    rpc_test! (
        ParitySet:parity_set_chain, "kovan"
        => "setChain", vec![r#""kovan""#];
        Value::Bool(true) => true
    );
}
