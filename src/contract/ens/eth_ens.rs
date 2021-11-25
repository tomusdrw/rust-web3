use crate::{
    api::Namespace,
    contract::ens::{public_resolver::PublicResolver, registry::Registry, reverse_resolver::ReverseResolver},
    signing::namehash,
    types::{Address, TransactionId, U256},
    Transport, Web3,
};
use hex::ToHex;
use idna::Config;

type ContractError = crate::contract::Error;
type EthError = crate::ethabi::Error;

const ADDR_INTERFACE_ID: &[u8; 4] = &[0x3b, 0x3b, 0x57, 0xde];
const BLOCKCHAIN_ADDR_INTERFACE_ID: &[u8; 4] = &[0xf1, 0xcb, 0x7e, 0x06];
const PUBKEY_INTERFACE_ID: &[u8; 4] = &[0xc8, 0x69, 0x02, 0x33];
const TEXT_INTERFACE_ID: &[u8; 4] = &[0x59, 0xd1, 0xd4, 0x3c];
const CONTENTHASH_INTERFACE_ID: &[u8; 4] = &[0xbc, 0x1c, 0x58, 0xd1];

/// Ethereum Name Service interface.
#[derive(Clone)]
pub struct Ens<T: Transport> {
    web3: Web3<T>,
    registry: Registry<T>,
    idna: Config,
    transport: T,
}

impl<T: Transport> Namespace<T> for Ens<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        let web3 = Web3::new(transport.clone());

        let registry = Registry::new(web3.eth());

        let idna = Config::default()
            .transitional_processing(false)
            .use_std3_ascii_rules(true);

        Self {
            transport,
            web3,
            registry,
            idna,
        }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Ens<T> {
    /// Normalize a domain name for namehash processing.
    ///
    /// [Specification](https://docs.ens.domains/contract-api-reference/name-processing#normalising-names)
    fn normalize_name(&self, domain: &str) -> Result<String, ContractError> {
        self.idna
            .to_ascii(domain)
            .map_err(|_| ContractError::Abi(EthError::InvalidData))
    }

    /*** Main ENS Registry Functions Below ***/

    /// Returns the owner of a name.
    pub async fn owner(&self, domain: &str) -> Result<Address, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        self.registry.owner(node).await
    }

    /// Returns the address of the resolver responsible for the name specified.
    pub async fn resolver(&self, domain: &str) -> Result<Address, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        self.registry.resolver(node).await
    }

    /// Returns the caching TTL (time-to-live) of a name.
    pub async fn ttl(&self, domain: &str) -> Result<u64, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        self.registry.ttl(node).await
    }

    /// Sets the owner of the given name.
    pub async fn set_owner(&self, from: Address, domain: &str, owner: Address) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        self.registry.set_owner(from, node, owner).await
    }

    /// Sets the resolver contract address of a name.
    pub async fn set_resolver(
        &self,
        from: Address,
        domain: &str,
        address: Address,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        self.registry.set_resolver(from, node, address).await
    }

    /// Sets the caching TTL (time-to-live) of a name.
    pub async fn set_ttl(&self, from: Address, domain: &str, ttl: u64) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        self.registry.set_ttl(from, node, ttl).await
    }

    /// Creates a new subdomain of the given node, assigning ownership of it to the specified owner.
    ///
    /// If the domain already exists, ownership is reassigned but the resolver and TTL are left unmodified.
    pub async fn set_subdomain_owner(
        &self,
        from: Address,
        domain: &str,
        subdomain: &str,
        owner: Address,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let label = self.normalize_name(subdomain)?;
        let label = crate::signing::keccak256(label.as_bytes());

        self.registry.set_subnode_owner(from, node, label, owner).await
    }

    /// Sets the owner, resolver, and TTL for an ENS record in a single operation.
    ///
    /// This function is offered for convenience, and is exactly equivalent to calling set_resolver, set_ttl and set_owner in that order.
    pub async fn set_record(
        &self,
        from: Address,
        domain: &str,
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        self.registry.set_record(from, node, owner, resolver, ttl).await
    }

    /// Sets the owner, resolver and TTL for a subdomain, creating it if necessary.
    ///
    /// This function is offered for convenience, and permits setting all three fields without first transferring ownership of the subdomain to the caller.
    pub async fn set_subdomain_record(
        &self,
        from: Address,
        domain: &str,
        subdomain: &str,
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let label = self.normalize_name(subdomain)?;
        let label = crate::signing::keccak256(label.as_bytes());

        self.registry
            .set_subnode_record(from, node, label, owner, resolver, ttl)
            .await
    }

    /// Sets or clears an approval.
    ///
    /// Approved accounts can execute all ENS registry operations on behalf of the caller.
    pub async fn set_approval_for_all(
        &self,
        from: Address,
        operator: Address,
        approved: bool,
    ) -> Result<TransactionId, ContractError> {
        self.registry.set_approval_for_all(from, operator, approved).await
    }

    /// Returns true if the operator is approved to make ENS registry operations on behalf of the owner.
    pub async fn is_approved_for_all(&self, owner: Address, operator: Address) -> Result<bool, ContractError> {
        self.registry.check_approval(owner, operator).await
    }

    /// Returns true if domain exists in the ENS registry.
    ///
    /// This will return false for records that are in the legacy ENS registry but have not yet been migrated to the new one.
    pub async fn record_exists(&self, domain: &str) -> Result<bool, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        self.registry.check_record_existence(node).await
    }

    /*** Public Resolver Functions Below ***/

    /// Returns true if the related Public Resolver does support the given interfaceId.
    pub async fn supports_interface(&self, domain: &str, interface_id: [u8; 4]) -> Result<bool, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        resolver.check_interface_support(interface_id).await
    }

    /// Resolves an ENS name to an Ethereum address.
    pub async fn eth_address(&self, domain: &str) -> Result<Address, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*ADDR_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.ethereum_address(node).await
    }

    /// Sets the address of an ENS name in this resolver.
    pub async fn set_eth_address(
        &self,
        from: Address,
        domain: &str,
        address: Address,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        resolver.set_ethereum_address(from, node, address).await
    }

    /// Returns the Blockchain address associated with the provided node and coinType, or 0 if none.
    pub async fn blockchain_address(&self, domain: &str, coin_type: U256) -> Result<Vec<u8>, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*BLOCKCHAIN_ADDR_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.blockchain_address(node, coin_type).await
    }

    /// Sets the blockchain address associated with the provided node and coinType to addr.
    pub async fn set_blockchain_address(
        &self,
        from: Address,
        domain: &str,
        coin_type: U256,
        a: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        resolver.set_blockchain_address(from, node, coin_type, a).await
    }

    /// Returns the X and Y coordinates of the curve point for the public key.
    pub async fn pubkey(&self, domain: &str) -> Result<([u8; 32], [u8; 32]), ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*PUBKEY_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.public_key(node).await
    }

    /// Sets the SECP256k1 public key associated with an ENS node.
    pub async fn set_pubkey(
        &self,
        from: Address,
        domain: &str,
        x: [u8; 32],
        y: [u8; 32],
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        resolver.set_public_key(from, node, x, y).await
    }

    /// Returns the content hash object associated with an ENS node.
    pub async fn content_hash(&self, domain: &str) -> Result<Vec<u8>, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*CONTENTHASH_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.content_hash(node).await
    }

    /// Sets the content hash associated with an ENS node.
    pub async fn set_content_hash(
        &self,
        from: Address,
        domain: &str,
        hash: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*CONTENTHASH_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        // https://eips.ethereum.org/EIPS/eip-1577
        if !(hash[0] == 0xe3 || hash[0] == 0xe4) {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.set_content_hash(from, node, hash).await
    }

    /// Returns the text record for a given key for the current ENS name.
    pub async fn text(&self, domain: &str, key: String) -> Result<String, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*TEXT_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.text_data(node, key).await
    }

    /// Sets the text record for a given key for the current ENS name.
    pub async fn set_text(
        &self,
        from: Address,
        domain: &str,
        key: String,
        value: String,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*TEXT_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.set_text_data(from, node, key, value).await
    }

    /// Returns the reverse record for a particular Ethereum address.
    pub async fn canonical_name(&self, from: Address) -> Result<String, ContractError> {
        let mut hex: String = from.encode_hex();
        hex.push_str(".addr.reverse");

        let node = namehash(&hex);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = ReverseResolver::new(self.web3.eth(), resolver_addr);

        // The reverse resolver does not support checking interfaces yet.
        /* if !resolver.check_interface_support(*NAME_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        } */

        resolver.canonical_name(node).await
    }

    /// Sets the reverse record for the current Ethereum address
    pub async fn set_canonical_name(
        &self,
        from: Address,
        domain: &str,
        name: String,
    ) -> Result<TransactionId, ContractError> {
        let domain = self.normalize_name(domain)?;
        let node = namehash(&domain);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = ReverseResolver::new(self.web3.eth(), resolver_addr);

        // The reverse resolver does not support checking interfaces yet.
        /* if !resolver.check_interface_support(*NAME_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        } */

        resolver.set_canonical_name(from, node, name).await
    }
}
