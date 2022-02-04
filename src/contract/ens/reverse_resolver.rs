//! Reverse Resolver ENS contract interface.

use crate::{
    api::Eth,
    contract::{Contract, Options},
    signing::NameHash,
    types::{Address, TransactionId},
    Transport,
};

type ContractError = crate::contract::Error;

/// Reverse resolution in ENS - the process of mapping from an Ethereum address (eg, 0x1234...) to an ENS name - is handled using a special namespace, *.addr.reverse*.
/// A special-purpose registrar controls this namespace and allocates subdomains to any caller based on their address.
///
/// For example, the account *0x314159265dd8dbb310642f98f50c066173c1259b* can claim *314159265dd8dbb310642f98f50c066173c1259b.addr.reverse*. After doing so, it can configure a resolver and expose metadata, such as a canonical ENS name for this address.
///
/// The reverse registrar provides functions to claim a reverse record, as well as a convenience function to configure the record as it's most commonly used, as a way of specifying a canonical name for an address.
///
/// The reverse registrar is specified in [EIP 181](https://eips.ethereum.org/EIPS/eip-181).
///
/// [Source](https://github.com/ensdomains/ens/blob/master/contracts/ReverseRegistrar.sol)
#[derive(Debug, Clone)]
pub struct ReverseResolver<T: Transport> {
    contract: Contract<T>,
}

impl<T: Transport> ReverseResolver<T> {
    /// Creates new instance of [`ReverseResolver`] given contract address.
    pub fn new(eth: Eth<T>, resolver_addr: Address) -> Self {
        // See https://github.com/ensdomains/ens-contracts for up to date contracts.
        let bytes = include_bytes!("DefaultReverseResolver.json");

        let contract = Contract::from_json(eth, resolver_addr, bytes).expect("Contract Creation Failed");

        Self { contract }
    }
}

impl<T: Transport> ReverseResolver<T> {
    /// Returns the canonical ENS name associated with the provided node.
    pub async fn canonical_name(&self, node: NameHash) -> Result<String, ContractError> {
        let options = Options::default();

        self.contract.query("name", node, None, options, None).await
    }

    /// Sets the canonical ENS name for the provided node to name.
    pub async fn set_canonical_name(
        &self,
        from: Address,
        node: NameHash,
        name: String,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setName", (node, name), from, options).await?;

        Ok(TransactionId::Hash(id))
    }
}
