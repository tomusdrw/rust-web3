//! ENS Registry contract interface.

use crate::{
    api::Eth,
    contract::{Contract, Options},
    signing::NameHash,
    types::{Address, TransactionId},
    Transport,
};

type ContractError = crate::contract::Error;

const ENS_REGISTRY_ADDRESS: &str = "00000000000C2E074eC69A0dFb2997BA6C7d2e1e";

/// The ENS registry is the core contract that lies at the heart of ENS resolution.
///
/// All ENS lookups start by querying the registry.
/// The registry maintains a list of domains, recording the owner, resolver, and TTL for each, and allows the owner of a domain to make changes to that data.
///
/// The ENS registry is specified in [EIP 137](https://eips.ethereum.org/EIPS/eip-137).
///
/// [Source](https://github.com/ensdomains/ens/blob/master/contracts/ENS.sol)
#[derive(Debug, Clone)]
pub struct Registry<T: Transport> {
    contract: Contract<T>,
}

impl<T: Transport> Registry<T> {
    /// Creates new instance of [`Registry`].
    pub fn new(eth: Eth<T>) -> Self {
        let address = ENS_REGISTRY_ADDRESS.parse().expect("Parsing Address");

        // See https://github.com/ensdomains/ens-contracts for up to date contracts.
        let json = include_bytes!("ENSRegistry.json");

        let contract = Contract::from_json(eth, address, json).expect("Contract Creation");

        Self { contract }
    }
}

impl<T: Transport> Registry<T> {
    /// Returns the owner of the name specified by node.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#get-owner)
    pub async fn owner(&self, node: NameHash) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("owner", node, None, options, None).await
    }

    /// Returns the address of the resolver responsible for the name specified by node.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#get-resolver)
    pub async fn resolver(&self, node: NameHash) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("resolver", node, None, options, None).await
    }

    /// Returns the caching time-to-live of the name specified by node.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#get-ttl)
    pub async fn ttl(&self, node: NameHash) -> Result<u64, ContractError> {
        let options = Options::default();

        self.contract.query("ttl", node, None, options, None).await
    }

    /// Reassigns ownership of the name identified by node to owner.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#set-owner)
    pub async fn set_owner(
        &self,
        from: Address,
        node: NameHash,
        owner: Address,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setOwner", (node, owner), from, options).await?;

        Ok(TransactionId::Hash(id))
    }

    /// Updates the resolver associated with the name identified by node to resolver.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#set-resolver)
    pub async fn set_resolver(
        &self,
        from: Address,
        node: NameHash,
        resolver: Address,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setResolver", (node, resolver), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }

    /// Updates the caching time-to-live of the name identified by node.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#set-ttl)
    pub async fn set_ttl(&self, from: Address, node: NameHash, ttl: u64) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setTTL", (node, ttl), from, options).await?;

        Ok(TransactionId::Hash(id))
    }

    /// Creates a new subdomain of node, assigning ownership of it to the specified owner.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#set-subdomain-owner)
    pub async fn set_subnode_owner(
        &self,
        from: Address,
        node: NameHash,
        label: [u8; 32],
        owner: Address,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setSubnodeOwner", (node, label, owner), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }

    /// Sets the owner, resolver, and TTL for an ENS record in a single operation.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#set-record)
    pub async fn set_record(
        &self,
        from: Address,
        node: NameHash,
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setRecord", (node, owner, resolver, ttl), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }

    /// Sets the owner, resolver and TTL for a subdomain, creating it if necessary.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#set-subdomain-record)
    pub async fn set_subnode_record(
        &self,
        from: Address,
        node: NameHash,
        label: [u8; 32],
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setSubnodeRecord", (node, label, owner, resolver, ttl), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }

    /// Sets or clears an approval.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#set-approval)
    pub async fn set_approval_for_all(
        &self,
        from: Address,
        operator: Address,
        approved: bool,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setApprovalForAll", (operator, approved), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }

    /// Returns true if operator is approved to make ENS registry operations on behalf of owner.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#check-approval)
    pub async fn check_approval(&self, owner: Address, operator: Address) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract
            .query("isApprovedForAll", (owner, operator), None, options, None)
            .await
    }

    /// Returns true if node exists in this ENS registry.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/ens#check-record-existence)
    pub async fn check_record_existence(&self, node: NameHash) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract.query("recordExists", node, None, options, None).await
    }
}
