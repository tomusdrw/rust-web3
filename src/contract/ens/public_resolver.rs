//! Public Resolver ENS contract interface.

use crate::{
    api::Eth,
    contract::{Contract, Options},
    signing::NameHash,
    types::{Address, Bytes, TransactionId, U256},
    Transport,
};

type ContractError = crate::contract::Error;

/// [`PublicResolver`] implements a general-purpose ENS resolver that is suitable for most standard ENS use-cases.
///
/// The public resolver permits updates to ENS records by the owner of the corresponding name.
///
/// The public resolver implements the following EIPs:
/// - [EIP 137](https://eips.ethereum.org/EIPS/eip-137) Contract address interface.
/// - [EIP 165](https://eips.ethereum.org/EIPS/eip-165) Interface Detection.
/// - [EIP 181](https://eips.ethereum.org/EIPS/eip-181) - Reverse resolution.
/// - [EIP 205](https://eips.ethereum.org/EIPS/eip-205) - ABI support.
/// - [EIP 619](https://github.com/ethereum/EIPs/pull/619) - SECP256k1 public keys.
/// - [EIP 634](https://eips.ethereum.org/EIPS/eip-634) - Text records.
/// - [EIP 1577](https://eips.ethereum.org/EIPS/eip-1577) - Content hash support.
/// - [EIP 2304](https://eips.ethereum.org/EIPS/eip-2304) - Multicoin support.
///
/// While the [`PublicResolver`] provides a convenient default implementation, many resolver implementations and versions exist.
/// Callers must not assume that a domain uses the current version of the public resolver, or that all of the methods described here are present.
/// To check if a resolver supports a feature, see [`check_interface_support`](#method.check_interface_support).
///
/// [Source](https://github.com/ensdomains/resolvers/blob/master/contracts/Resolver.sol)
#[derive(Debug, Clone)]
pub struct PublicResolver<T: Transport> {
    contract: Contract<T>,
}

impl<T: Transport> PublicResolver<T> {
    /// Creates new instance of [`PublicResolver`] given contract address.
    pub fn new(eth: Eth<T>, resolver_addr: Address) -> Self {
        // See https://github.com/ensdomains/ens-contracts for up to date contracts.
        let bytes = include_bytes!("PublicResolver.json");

        let contract = Contract::from_json(eth, resolver_addr, bytes).expect("Contract Creation");

        Self { contract }
    }
}

impl<T: Transport> PublicResolver<T> {
    /// ENS uses ERC 165 for interface detection.
    /// ERC 165 requires that supporting contracts implement a function, supportsInterface, which takes an interface ID and returns a boolean value indicating if this interface is supported or not.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#check-interface-support)
    pub async fn check_interface_support(&self, interface_id: [u8; 4]) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract
            .query("supportsInterface", interface_id, None, options, None)
            .await
    }

    /// Returns the Ethereum address associated with the provided node, or 0 if none.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#get-ethereum-address)
    pub async fn ethereum_address(&self, node: NameHash) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("addr", node, None, options, None).await
    }

    /// Sets the Ethereum address associated with the provided node to addr.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#set-ethereum-address)
    pub async fn set_ethereum_address(
        &self,
        from: Address,
        node: NameHash,
        address: Address,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setAddr", (node, address), from, options).await?;

        Ok(TransactionId::Hash(id))
    }

    /// Returns the Blockchain address associated with the provided node and coinType, or 0 if none.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#get-blockchain-address)
    pub async fn blockchain_address(&self, node: NameHash, coin_type: U256) -> Result<Vec<u8>, ContractError> {
        let options = Options::default();

        self.contract
            .query("addr", (node, coin_type), None, options, None)
            .await
    }

    /// Sets the blockchain address associated with the provided node and coinType to addr.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#set-blockchain-address)
    pub async fn set_blockchain_address(
        &self,
        from: Address,
        node: NameHash,
        coin_type: U256,
        a: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setAddr", (node, coin_type, a), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }

    /// Returns the canonical ENS name associated with the provided node.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#get-canonical-name)
    pub async fn canonical_name(&self, node: NameHash) -> Result<String, ContractError> {
        let options = Options::default();

        self.contract.query("name", node, None, options, None).await
    }

    /// Sets the canonical ENS name for the provided node to name.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#set-canonical-name)
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

    /// Returns the content hash for node, if one exists.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#get-content-hash)
    pub async fn content_hash(&self, node: NameHash) -> Result<Vec<u8>, ContractError> {
        let options = Options::default();

        self.contract.query("contenthash", node, None, options, None).await
    }

    /// Sets the content hash for the provided node to hash.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#set-content-hash)
    pub async fn set_content_hash(
        &self,
        from: Address,
        node: NameHash,
        hash: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setContenthash", (node, hash), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }

    /// Returns a matching ABI definition for the provided node, if one exists.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#get-contract-abi)
    pub async fn abi(&self, node: NameHash, content_types: U256) -> Result<(U256, Vec<u8>), ContractError> {
        let options = Options::default();

        self.contract
            .query("ABI", (node, content_types), None, options, None)
            .await
    }

    /// Sets or updates ABI data for node.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#set-contract-abi)
    pub async fn set_contract_abi(
        &self,
        from: Address,
        node: NameHash,
        content_type: U256,
        data: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setABI", (node, content_type, data), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }

    /// Returns the ECDSA SECP256k1 public key for node, as a 2-tuple (x, y).
    /// If no public key is set, (0, 0) is returned.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#get-public-key)
    pub async fn public_key(&self, node: NameHash) -> Result<([u8; 32], [u8; 32]), ContractError> {
        let options = Options::default();

        self.contract.query("pubkey", node, None, options, None).await
    }

    /// Sets the ECDSA SECP256k1 public key for node to (x, y).
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#set-public-key)
    pub async fn set_public_key(
        &self,
        from: Address,
        node: NameHash,
        x: [u8; 32],
        y: [u8; 32],
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setPubkey", (node, x, y), from, options).await?;

        Ok(TransactionId::Hash(id))
    }

    /// This function is not explained anywhere. More info needed!
    pub async fn dnsrr(&self, node: NameHash) -> Result<Bytes, ContractError> {
        let options = Options::default();

        self.contract.query("dnsrr", node, None, options, None).await
    }

    /// Retrieves text metadata for node.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#get-public-key)
    pub async fn text_data(&self, node: NameHash, key: String) -> Result<String, ContractError> {
        let options = Options::default();

        self.contract.query("text", (node, key), None, options, None).await
    }

    /// Sets text metadata for node with the unique key key to value, overwriting anything previously stored for node and key.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#set-text-data)
    pub async fn set_text_data(
        &self,
        from: Address,
        node: NameHash,
        key: String,
        value: String,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setText", (node, key, value), from, options).await?;

        Ok(TransactionId::Hash(id))
    }

    /// Permits users to set multiple records in a single operation.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/publicresolver#multicall)
    pub async fn multicall(&self, data: Bytes) -> Result<Bytes, ContractError> {
        let options = Options::default();

        self.contract.query("multicall", data, None, options, None).await
    }

    /// This function is not explained anywhere. More info needed!
    pub async fn interface_implementer(&self, node: NameHash, interface: [u8; 4]) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract
            .query("interfaceImplementer", (node, interface), None, options, None)
            .await
    }

    /// This function is not explained anywhere. More info needed!
    pub async fn set_dnsrr(
        &self,
        from: Address,
        node: NameHash,
        data: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setDnsrr", (node, data), from, options).await?;

        Ok(TransactionId::Hash(id))
    }

    /// This function is not explained anywhere. More info needed!
    pub async fn set_interface(
        &self,
        from: Address,
        node: NameHash,
        interface: [u8; 4],
        implementer: Address,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self
            .contract
            .call("setInterface", (node, interface, implementer), from, options)
            .await?;

        Ok(TransactionId::Hash(id))
    }
}
