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
    /// Create a new ENS interface with the given transport.
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

    /// Returns the owner of the name specified by ```node```.
    pub async fn owner(&self, node: &str) -> Result<Address, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        self.registry.owner(node).await
    }

    /// Returns the address of the resolver responsible for the name specified by ```node```.
    pub async fn resolver(&self, node: &str) -> Result<Address, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        self.registry.resolver(node).await
    }

    /// Returns the caching time-to-live of the name specified by ```node```.
    ///
    /// Systems that wish to cache information about a name, including ownership, resolver address, and records, should respect this value.
    ///
    /// If TTL is zero, new data should be fetched on each query.
    pub async fn ttl(&self, node: &str) -> Result<u64, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        self.registry.ttl(node).await
    }

    /// Reassigns ownership of the name identified by ```node``` to ```owner```.
    ///
    /// Only callable by the current owner of the name.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event Transfer(bytes32 indexed node, address owner);
    /// ```
    pub async fn set_owner(&self, from: Address, node: &str, owner: Address) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        self.registry.set_owner(from, node, owner).await
    }

    /// Updates the resolver associated with the name identified by ```node``` to ```resolver```.
    ///
    /// Only callable by the current owner of the name.
    /// ```resolver``` must specify the address of a contract that implements the Resolver interface.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event NewResolver(bytes32 indexed node, address resolver);
    /// ```
    pub async fn set_resolver(
        &self,
        from: Address,
        node: &str,
        address: Address,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        self.registry.set_resolver(from, node, address).await
    }

    /// Updates the caching time-to-live of the name identified by ```node```.
    ///
    /// Only callable by the current owner of the name.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event NewTTL(bytes32 indexed node, uint64 ttl);
    /// ```
    pub async fn set_ttl(&self, from: Address, node: &str, ttl: u64) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        self.registry.set_ttl(from, node, ttl).await
    }

    /// Creates a new subdomain of ```node```, assigning ownership of it to the specified ```owner```.
    ///
    /// If the domain already exists, ownership is reassigned but the resolver and TTL are left unmodified.
    ///
    /// For example, if you own alice.eth and want to create the subdomain iam.alice.eth, supply ```alice.eth``` as the ```node```, and ```iam``` as the ```label```.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event NewOwner(bytes32 indexed node, bytes32 indexed label, address owner);
    /// ```
    pub async fn set_subdomain_owner(
        &self,
        from: Address,
        node: &str,
        label: &str,
        owner: Address,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let label = self.normalize_name(label)?;
        let label = crate::signing::keccak256(label.as_bytes());

        self.registry.set_subnode_owner(from, node, label, owner).await
    }

    /// Sets the owner, resolver, and TTL for an ENS record in a single operation.
    ///
    /// This function is offered for convenience, and is exactly equivalent to calling [`set_resolver`](#method.set_resolver), [`set_ttl`](#method.set_ttl) and [`set_owner`](#method.set_owner) in that order.
    pub async fn set_record(
        &self,
        from: Address,
        node: &str,
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        self.registry.set_record(from, node, owner, resolver, ttl).await
    }

    /// Sets the owner, resolver and TTL for a subdomain, creating it if necessary.
    ///
    /// This function is offered for convenience, and permits setting all three fields without first transferring ownership of the subdomain to the caller.
    pub async fn set_subdomain_record(
        &self,
        from: Address,
        node: &str,
        label: &str,
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let label = self.normalize_name(label)?;
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

    /// Returns true if ```operator``` is approved to make ENS registry operations on behalf of ```owner```.
    pub async fn is_approved_for_all(&self, owner: Address, operator: Address) -> Result<bool, ContractError> {
        self.registry.check_approval(owner, operator).await
    }

    /// Returns true if ```node``` exists in this ENS registry.
    ///
    /// This will return false for records that are in the legacy ENS registry but have not yet been migrated to the new one.
    pub async fn record_exists(&self, node: &str) -> Result<bool, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        self.registry.check_record_existence(node).await
    }

    /*** Public Resolver Functions Below ***/

    /// ENS uses [ERC 165](https://eips.ethereum.org/EIPS/eip-165) for interface detection.
    ///
    /// ERC 165 requires that supporting contracts implement a function, ```supportsInterface```, which takes an interface ID and returns a boolean value indicating if this interface is supported or not.
    /// Interface IDs are calculated as the exclusive-or of the four-byte function identifiers of each function included in the interface.
    ///
    /// For example, ```addr(bytes32)``` has the function ID *0x3b3b57de*.
    /// Because it is the only function in the Ethereum Address interface, its interface ID is also *0x3b3b57de*, and so calling ```supportsInterface(0x3b3b57de)``` will return *true* for any resolver that supports ```addr()```.
    /// ERC 165 has an interface ID of *0x01ffc9a7*, so ```supportsInterface(0x01ffc9a7)``` will always return *true* for any ERC 165 supporting contract (and hence for any resolver).
    ///
    /// Note that the public resolver does not expose explicit interfaces for setter functions, so there are no automated means to check for support for a given setter function.
    pub async fn supports_interface(&self, node: &str, interface_id: [u8; 4]) -> Result<bool, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        resolver.check_interface_support(interface_id).await
    }

    /// Returns the Ethereum address associated with the provided ```node```, or 0 if none.
    ///
    /// This function has interface ID *0x3b3b57de*.
    ///
    /// This function is specified in [EIP 137](https://eips.ethereum.org/EIPS/eip-137).
    pub async fn eth_address(&self, node: &str) -> Result<Address, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*ADDR_INTERFACE_ID).await? {
            return Err(ContractError::InterfaceUnsupported);
        }

        resolver.ethereum_address(node).await
    }

    /// Sets the Ethereum address associated with the provided ```node``` to ```addr```.
    ///
    /// Only callable by the owner of ```node```.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event AddrChanged(bytes32 indexed node, address a);
    /// ```
    pub async fn set_eth_address(
        &self,
        from: Address,
        node: &str,
        addr: Address,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        resolver.set_ethereum_address(from, node, addr).await
    }

    /// Returns the Blockchain address associated with the provided ```node``` and ```coin_type```, or 0 if none.
    ///
    /// This function has interface ID *0xf1cb7e06*.
    ///
    /// This function is specified in [EIP 2304](https://eips.ethereum.org/EIPS/eip-2304).
    ///
    /// The return value is the cryptocurency address in its native binary format and each blockchain address has a different encoding and decoding method.
    ///
    /// For example, the Bitcoin address ```1A1zP1eP5QGefi2DMPTfTL5SLmv7DivfNa``` base58check decodes to the 21 bytes ```0062e907b15cbf27d5425399ebf6f0fb50ebb88f18``` then scriptPubkey encodes to 25 bytes ```76a91462e907b15cbf27d5425399ebf6f0fb50ebb88f1888ac``` whereas the BNB address ```bnb1grpf0955h0ykzq3ar5nmum7y6gdfl6lxfn46h2``` Bech32 decodes to the binary representation ```40c2979694bbc961023d1d27be6fc4d21a9febe6```.
    ///
    /// A zero-length string will be returned if the specified coin ID does not exist on the specified node.
    pub async fn blockchain_address(&self, node: &str, coin_type: U256) -> Result<Vec<u8>, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*BLOCKCHAIN_ADDR_INTERFACE_ID).await? {
            return Err(ContractError::InterfaceUnsupported);
        }

        resolver.blockchain_address(node, coin_type).await
    }

    /// Sets the blockchain address associated with the provided ```node``` and ```coin_type``` to ```addr```.
    ///
    /// ```coinType``` is the cryptocurrency coin type index from [SLIP44](https://github.com/satoshilabs/slips/blob/master/slip-0044.md).
    ///
    /// Only callable by the owner of ```node```.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event AddressChanged(bytes32 indexed node, uint coinType, bytes newAddress);
    /// ```
    pub async fn set_blockchain_address(
        &self,
        from: Address,
        node: &str,
        coin_type: U256,
        addr: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        resolver.set_blockchain_address(from, node, coin_type, addr).await
    }

    /// Returns the ECDSA SECP256k1 public key for ```node```, as a 2-tuple ```(x, y)```.
    /// If no public key is set, ```(0, 0)``` is returned.
    ///
    /// This function has interface ID *0xc8690233*.
    ///
    /// This function is specified in [EIP 619](https://github.com/ethereum/EIPs/issues/619).
    pub async fn pubkey(&self, node: &str) -> Result<([u8; 32], [u8; 32]), ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*PUBKEY_INTERFACE_ID).await? {
            return Err(ContractError::InterfaceUnsupported);
        }

        resolver.public_key(node).await
    }

    /// Sets the ECDSA SECP256k1 public key for ```node``` to ```(x, y)```.
    ///
    /// Only callable by the owner of node.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event PubkeyChanged(bytes32 indexed node, bytes32 x, bytes32 y);
    /// ```
    pub async fn set_pubkey(
        &self,
        from: Address,
        node: &str,
        x: [u8; 32],
        y: [u8; 32],
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        resolver.set_public_key(from, node, x, y).await
    }

    /// Returns the content hash for ```node```, if one exists.
    ///
    /// Values are formatted as machine-readable [multicodecs](https://github.com/multiformats/multicodec), as specified in [EIP 1577](https://eips.ethereum.org/EIPS/eip-1577).
    ///
    /// ```content_hash``` is used to store IPFS and Swarm content hashes, which permit resolving ENS addresses to distributed content (eg, websites) hosted on these distributed networks.
    ///
    /// This function has interface ID *0xbc1c58d1*.
    ///
    /// This function is specified in [EIP 1577](https://eips.ethereum.org/EIPS/eip-1157).
    pub async fn content_hash(&self, node: &str) -> Result<Vec<u8>, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*CONTENTHASH_INTERFACE_ID).await? {
            return Err(ContractError::InterfaceUnsupported);
        }

        resolver.content_hash(node).await
    }

    /// Sets the content hash for the provided ```node``` to ```hash```.
    ///
    /// Only callable by the owner of ```node```.
    ///
    /// Values are formatted as machine-readable [multicodecs](https://github.com/multiformats/multicodec), as specified in [EIP 1577](https://eips.ethereum.org/EIPS/eip-1577).
    ///
    /// Emits the following event:
    /// ```solidity
    /// event ContenthashChanged(bytes32 indexed node, bytes hash);
    /// ```
    pub async fn set_content_hash(
        &self,
        from: Address,
        node: &str,
        hash: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*CONTENTHASH_INTERFACE_ID).await? {
            return Err(ContractError::InterfaceUnsupported);
        }

        // https://eips.ethereum.org/EIPS/eip-1577
        if !(hash[0] == 0xe3 || hash[0] == 0xe4) {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.set_content_hash(from, node, hash).await
    }

    /// Retrieves text metadata for ```node```.
    /// Each name may have multiple pieces of metadata, identified by a unique string key.
    /// If no text data exists for ```node``` with the key ```key```, the empty string is returned.
    ///
    /// Standard values for ```key``` are:
    ///
    /// | key         | Meaning                                                                                                                                                   |
    /// |-------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------|
    /// | email       | An email address                                                                                                                                          |
    /// | url         | A URL                                                                                                                                                     |
    /// | avatar      | A URL to an image used as an avatar or logo                                                                                                               |
    /// | description | A description of the name                                                                                                                                 |
    /// | notice      | A notice regarding this name                                                                                                                              |     
    /// | keywords    | A list of comma-separated keywords, ordered by most significant first; clients that interpresent this field may choose a threshold beyond which to ignore |
    ///
    /// In addition, anyone may specify vendor-specific keys, which must be prefixed with ```vnd.```. The following vendor-specific keys are currently known:
    ///
    /// | key         | Meaning                                                                                                                                                   |
    /// |-------------|-----------------|
    /// | com.twitter | Twitter handle  |                                                                                                                                         |
    /// | com.github  | Github username |
    ///
    /// This function has interface ID *0x59d1d43c*.
    ///
    /// This function is specified in [EIP 634]().
    pub async fn text(&self, node: &str, key: String) -> Result<String, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*TEXT_INTERFACE_ID).await? {
            return Err(ContractError::InterfaceUnsupported);
        }

        resolver.text_data(node, key).await
    }

    /// Sets text metadata for ```node``` with the unique key ```key``` to ```value```, overwriting anything previously stored for ```node``` and ```key```.
    /// To clear a text field, set it to the empty string.
    ///
    /// Only callable by the owner of ```node```.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event TextChanged(bytes32 indexed node, string indexedKey, string key);
    /// ```
    pub async fn set_text(
        &self,
        from: Address,
        node: &str,
        key: String,
        value: String,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = PublicResolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*TEXT_INTERFACE_ID).await? {
            return Err(ContractError::InterfaceUnsupported);
        }

        resolver.set_text_data(from, node, key, value).await
    }

    /*** Reverse Resolver Functions Below ***/

    /// Returns the canonical ENS name associated with the provided ```addr```.
    /// Used exclusively for reverse resolution.
    ///
    /// This function has interface ID *0x691f3431*.
    ///
    /// This function is specified in [EIP 181](https://eips.ethereum.org/EIPS/eip-181).
    pub async fn canonical_name(&self, addr: Address) -> Result<String, ContractError> {
        let mut hex: String = addr.encode_hex();
        hex.push_str(".addr.reverse");

        let node = namehash(&hex);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = ReverseResolver::new(self.web3.eth(), resolver_addr);

        // The reverse resolver does not support checking interfaces yet.
        /* if !resolver.check_interface_support(*NAME_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::Other("Interface Unsupported".into())));
        } */

        resolver.canonical_name(node).await
    }

    /// Sets the canonical ENS name for the provided ```node``` to ```name```.
    ///
    /// Only callable by the owner of ```node```.
    ///
    /// Emits the following event:
    /// ```solidity
    /// event NameChanged(bytes32 indexed node, string name);
    /// ```
    pub async fn set_canonical_name(
        &self,
        from: Address,
        node: &str,
        name: String,
    ) -> Result<TransactionId, ContractError> {
        let node = self.normalize_name(node)?;
        let node = namehash(&node);

        let resolver_addr = self.registry.resolver(node).await?;
        let resolver = ReverseResolver::new(self.web3.eth(), resolver_addr);

        // The reverse resolver does not support checking interfaces yet.
        /* if !resolver.check_interface_support(*NAME_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::Other("Interface Unsupported".into())));
        } */

        resolver.set_canonical_name(from, node, name).await
    }
}
