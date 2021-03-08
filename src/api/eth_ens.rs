//! `Eth` namespace, ens

use crate::{
    api::Eth,
    api::Namespace,
    contract::{Contract, Options},
    signing::namehash,
    types::{Address, Bytes, H256, U256},
    Transport, Web3,
};

type ContractError = crate::contract::Error;
type EthError = crate::ethabi::Error;

use idna::Config;

const ENS_REGISTRY_ADDRESS: &str = "00000000000C2E074eC69A0dFb2997BA6C7d2e1e";

const ADDR_INTERFACE_ID: &[u8; 4] = &[0x3b, 0x3b, 0x57, 0xde];
const BLOCKCHAIN_ADDR_INTERFACE_ID: &[u8; 4] = &[0xf1, 0xcb, 0x7e, 0x06];
const NAME_INTERFACE_ID: &[u8; 4] = &[0x69, 0x1f, 0x34, 0x31];
const ABI_INTERFACE_ID: &[u8; 4] = &[0x22, 0x03, 0xab, 0x56];
const PUBKEY_INTERFACE_ID: &[u8; 4] = &[0xc8, 0x69, 0x02, 0x33];
const TEXT_INTERFACE_ID: &[u8; 4] = &[0x59, 0xd1, 0xd4, 0x3c];
const CONTENTHASH_INTERFACE_ID: &[u8; 4] = &[0xbc, 0x1c, 0x58, 0xd1];

/// `Eth` namespace, ens
#[derive(Clone)]
pub struct Ens<T: Transport> {
    web3: Web3<T>,
    registry: Registry<T>,
    idna: Config,
    transport: T,
}

//TODO fix initialization mess

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
    /// Sets the resolver contract address of a name.
    pub async fn set_resolver(&self, from: Address, domain: &str, address: Address) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        self.registry.set_resolver(from, node, address).await
    }

    /// Returns the owner of a name.
    pub async fn get_owner(&self, domain: &str) -> Result<Address, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        self.registry.get_owner(node).await
    }

    /// Sets the owner of the given name.
    pub async fn set_owner(&self, from: Address, domain: &str, owner: Address) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        self.registry.set_owner(from, node, owner).await
    }

    /// Returns the caching TTL (time-to-live) of a name.
    pub async fn get_ttl(&self, domain: &str) -> Result<u64, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        self.registry.get_ttl(node).await
    }

    /// Sets the caching TTL (time-to-live) of a name.
    pub async fn set_ttl(&self, from: Address, domain: &str, ttl: u64) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        self.registry.set_ttl(from, node, ttl).await
    }

    /// Creates a new subdomain of the given node, assigning ownership of it to the specified owner.
    pub async fn set_subnode_owner(
        &self,
        from: Address,
        domain: &str,
        label: &str,
        owner: Address,
    ) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");
        let node = namehash(&domain);

        let label = self.idna.to_ascii(label).expect("Cannot Normalize"); //Do we have to normalize here?
        let label = crate::signing::keccak256(label.as_bytes());

        self.registry.set_subnode_owner(from, node, label, owner).await
    }

    /// Sets the owner, resolver, and TTL for an ENS record in a single operation.
    pub async fn set_record(
        &self,
        from: Address,
        domain: &str,
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");
        let node = namehash(&domain);

        self.registry.set_record(from, node, owner, resolver, ttl).await
    }

    /// Sets the owner, resolver and TTL for a subdomain, creating it if necessary.
    pub async fn set_subnode_record(
        &self,
        from: Address,
        domain: &str,
        label: &str,
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");
        let node = namehash(&domain);

        let label = self.idna.to_ascii(label).expect("Cannot Normalize"); //Do we have to normalize here?
        let label = crate::signing::keccak256(label.as_bytes());

        self.registry
            .set_subnode_record(from, node, label, owner, resolver, ttl)
            .await
    }

    /// Sets or clears an approval.
    /// Approved accounts can execute all ENS registry operations on behalf of the caller.
    pub async fn set_approval_for_all(
        &self,
        from: Address,
        operator: Address,
        approved: bool,
    ) -> Result<H256, ContractError> {
        self.registry.set_approval_for_all(from, operator, approved).await
    }

    /// Returns true if the operator is approved to make ENS registry operations on behalf of the owner.
    pub async fn is_approved_for_all(&self, owner: Address, operator: Address) -> Result<bool, ContractError> {
        self.registry.check_approval(owner, operator).await
    }

    /// Returns true if node exists in this ENS registry.
    /// This will return false for records that are in the legacy ENS registry but have not yet been migrated to the new one.
    pub async fn record_exists(&self, domain: &str) -> Result<bool, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");
        let node = namehash(&domain);

        self.registry.check_record_existence(node).await
    }

    /// Resolves an ENS name to an Ethereum address.
    pub async fn get_eth_address(&self, domain: &str) -> Result<Address, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*ADDR_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.get_ethereum_address(node).await
    }

    /// Sets the address of an ENS name in this resolver.
    pub async fn set_eth_address(&self, from: Address, domain: &str, address: Address) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        resolver.set_ethereum_address(from, node, address).await
    }

    /// Returns the Blockchain address associated with the provided node and coinType, or 0 if none.
    /* pub async fn get_blockchain_address(&self, domain: &str, coin_type: U256) -> Result<Bytes, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*BLOCKCHAIN_ADDR_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.get_blockchain_address(node, coin_type).await
    } */

    /// Sets the blockchain address associated with the provided node and coinType to addr.
    /* pub async fn set_blockchain_address(
        &self,
        from: Address,
        domain: &str,
        coin_type: U256,
        a: Bytes,
    ) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        resolver.set_blockchain_address(from, node, coin_type, a).await
    } */

    /// Returns the X and Y coordinates of the curve point for the public key.
    pub async fn get_pubkey(&self, domain: &str) -> Result<([u8; 32], [u8; 32]), ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*PUBKEY_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.get_public_key(node).await
    }

    /// Sets the SECP256k1 public key associated with an ENS node.
    pub async fn set_pubkey(
        &self,
        from: Address,
        domain: &str,
        x: [u8; 32],
        y: [u8; 32],
    ) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        resolver.set_public_key(from, node, x, y).await
    }

    /// Returns the content hash object associated with an ENS node.
    pub async fn get_content_hash(&self, domain: &str) -> Result<Bytes, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        if !resolver.check_interface_support(*CONTENTHASH_INTERFACE_ID).await? {
            return Err(ContractError::Abi(EthError::InvalidData));
        }

        resolver.get_content_hash(node).await
    }

    /// Sets the content hash associated with an ENS node.
    /* pub async fn set_content_hash(&self, from: Address, domain: &str, hash: Bytes) -> Result<H256, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        //TODO check hash is valid multicodec

        resolver.set_content_hash(from, node, hash).await
    } */

    /// Returns true if the related Resolver does support the given interfaceId.
    pub async fn supports_interface(&self, domain: &str, interface_id: [u8; 4]) -> Result<bool, ContractError> {
        let domain = self.idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(node).await?;

        let resolver = Resolver::new(self.web3.eth(), resolver_addr);

        resolver.check_interface_support(interface_id).await
    }
}

#[derive(Debug, Clone)]
struct Registry<T: Transport> {
    contract: Contract<T>,
}

impl<T: Transport> Registry<T> {
    fn new(eth: Eth<T>) -> Self {
        let address = ENS_REGISTRY_ADDRESS.parse().expect("Parsing Address Failed");

        let contract =
            Contract::from_json(eth, address, REGISTRY_CONTRACT.as_bytes()).expect("Contract Creation Failed");

        Self { contract }
    }
}

impl<T: Transport> Registry<T> {
    // https://github.com/ensdomains/ens/blob/master/contracts/ENS.sol

    // https://docs.ens.domains/contract-api-reference/ens#set-record
    async fn set_record(
        &self,
        from: Address,
        node: [u8; 32],
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract
            .call("setRecord", (node, owner, resolver, ttl), from, options)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-subdomain-record
    async fn set_subnode_record(
        &self,
        from: Address,
        node: [u8; 32],
        label: [u8; 32],
        owner: Address,
        resolver: Address,
        ttl: u64,
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract
            .call("setSubnodeRecord", (node, label, owner, resolver, ttl), from, options)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-subdomain-owner
    async fn set_subnode_owner(
        &self,
        from: Address,
        node: [u8; 32],
        label: [u8; 32],
        owner: Address,
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract
            .call("setSubnodeOwner", (node, label, owner), from, options)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-resolver
    async fn set_resolver(&self, from: Address, node: [u8; 32], resolver: Address) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setResolver", (node, resolver), from, options).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-owner
    async fn set_owner(&self, from: Address, node: [u8; 32], owner: Address) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setOwner", (node, owner), from, options).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-ttl
    async fn set_ttl(&self, from: Address, node: [u8; 32], ttl: u64) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setTTL", (node, ttl), from, options).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-approval
    async fn set_approval_for_all(
        &self,
        from: Address,
        operator: Address,
        approved: bool,
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract
            .call("setApprovalForAll", (operator, approved), from, options)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/ens#get-owner
    async fn get_owner(&self, node: [u8; 32]) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("owner", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#get-resolver
    async fn get_resolver(&self, node: [u8; 32]) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("resolver", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#get-ttl
    async fn get_ttl(&self, node: [u8; 32]) -> Result<u64, ContractError> {
        let options = Options::default();

        self.contract.query("ttl", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#check-record-existence
    async fn check_record_existence(&self, node: [u8; 32]) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract.query("recordExists", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#check-approval
    async fn check_approval(&self, owner: Address, operator: Address) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract
            .query("isApprovedForAll", (owner, operator), None, options, None)
            .await
    }
}

#[derive(Debug, Clone)]
struct Resolver<T: Transport> {
    contract: Contract<T>,
}

impl<T: Transport> Resolver<T> {
    fn new(eth: Eth<T>, resolver_addr: Address) -> Self {
        let contract =
            Contract::from_json(eth, resolver_addr, RESOLVER_CONTRACT.as_bytes()).expect("Contract Creation Failed");

        Self { contract }
    }
}

impl<T: Transport> Resolver<T> {
    // https://github.com/ensdomains/resolvers/blob/master/contracts/Resolver.sol

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-contract-abi
    async fn get_abi(&self, node: [u8; 32], content_types: U256) -> Result<(U256, Bytes), ContractError> {
        let options = Options::default();

        self.contract
            .query("ABI", (node, content_types), None, options, None)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-ethereum-address
    async fn get_ethereum_address(&self, node: [u8; 32]) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("addr", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-blockchain-address
    async fn get_blockchain_address(&self, node: [u8; 32], coin_type: U256) -> Result<Bytes, ContractError> {
        let options = Options::default();

        self.contract
            .query("addr", (node, coin_type), None, options, None)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-content-hash
    async fn get_content_hash(&self, node: [u8; 32]) -> Result<Bytes, ContractError> {
        let options = Options::default();

        self.contract.query("contenthash", node, None, options, None).await
    }

    //dnsrr???

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-canonical-name
    async fn get_canonical_name(&self, node: [u8; 32]) -> Result<String, ContractError> {
        let options = Options::default();

        self.contract.query("name", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-public-key
    async fn get_public_key(&self, node: [u8; 32]) -> Result<([u8; 32], [u8; 32]), ContractError> {
        let options = Options::default();

        self.contract.query("pubkey", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-text-data
    async fn get_text_data(&self, node: [u8; 32], key: String) -> Result<String, ContractError> {
        let options = Options::default();

        self.contract.query("text", (node, key), None, options, None).await
    }

    //interfaceImplementer

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-contract-abi
    async fn set_contract_abi(
        &self,
        from: Address,
        node: [u8; 32],
        content_type: U256,
        data: Bytes,
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract
            .call("setABI", (node, content_type, data), from, options)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-ethereum-address
    async fn set_ethereum_address(
        &self,
        from: Address,
        node: [u8; 32],
        address: Address,
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setAddr", (node, address), from, options).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-blockchain-address
    async fn set_blockchain_address(
        &self,
        from: Address,
        node: [u8; 32],
        coin_type: U256,
        a: Bytes,
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setAddr", (node, coin_type, a), from, options).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-content-hash
    async fn set_content_hash(&self, from: Address, node: [u8; 32], hash: Bytes) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setContenthash", (node, hash), from, options).await
    }

    //setDnsrr

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-canonical-name
    async fn set_canonical_name(&self, from: Address, node: [u8; 32], name: String) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setName", (node, name), from, options).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-public-key
    async fn set_public_key(
        &self,
        from: Address,
        node: [u8; 32],
        x: [u8; 32],
        y: [u8; 32],
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setPubkey", (node, x, y), from, options).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-text-data
    async fn set_text_data(
        &self,
        from: Address,
        node: [u8; 32],
        key: String,
        value: String,
    ) -> Result<H256, ContractError> {
        let options = Options::default();

        self.contract.call("setText", (node, key, value), from, options).await
    }

    //setInterface

    // https://docs.ens.domains/contract-api-reference/publicresolver#check-interface-support
    async fn check_interface_support(&self, interface_id: [u8; 4]) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract
            .query("supportsInterface", interface_id, None, options, None)
            .await
    }

    //multicall
}

const REGISTRY_CONTRACT: &str = r#"[
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "resolver",
        "outputs": [
            {
                "name": "",
                "type": "address"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "owner",
        "outputs": [
            {
                "name": "",
                "type": "address"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "label",
                "type": "bytes32"
            },
            {
                "name": "owner",
                "type": "address"
            }
        ],
        "name": "setSubnodeOwner",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "ttl",
                "type": "uint64"
            }
        ],
        "name": "setTTL",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "ttl",
        "outputs": [
            {
                "name": "",
                "type": "uint64"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "resolver",
                "type": "address"
            }
        ],
        "name": "setResolver",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "owner",
                "type": "address"
            }
        ],
        "name": "setOwner",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "owner",
                "type": "address"
            }
        ],
        "name": "Transfer",
        "type": "event"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": true,
                "name": "label",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "owner",
                "type": "address"
            }
        ],
        "name": "NewOwner",
        "type": "event"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "resolver",
                "type": "address"
            }
        ],
        "name": "NewResolver",
        "type": "event"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "ttl",
                "type": "uint64"
            }
        ],
        "name": "NewTTL",
        "type": "event"
    },
    {
        "constant": false,
        "inputs": [
            {
                "internalType": "bytes32",
                "name": "node",
                "type": "bytes32"
            },
            {
                "internalType": "address",
                "name": "owner",
                "type": "address"
            },
            {
                "internalType": "address",
                "name": "resolver",
                "type": "address"
            },
            {
                "internalType": "uint64",
                "name": "ttl",
                "type": "uint64"
            }
        ],
        "name": "setRecord",
        "outputs": [],
        "payable": false,
        "stateMutability": "nonpayable",
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "internalType": "address",
                "name": "operator",
                "type": "address"
            },
            {
                "internalType": "bool",
                "name": "approved",
                "type": "bool"
            }
        ],
        "name": "setApprovalForAll",
        "outputs": [],
        "payable": false,
        "stateMutability": "nonpayable",
        "type": "function"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "internalType": "address",
                "name": "owner",
                "type": "address"
            },
            {
                "indexed": true,
                "internalType": "address",
                "name": "operator",
                "type": "address"
            },
            {
                "indexed": false,
                "internalType": "bool",
                "name": "approved",
                "type": "bool"
            }
        ],
        "name": "ApprovalForAll",
        "type": "event"
    },
    {
        "constant": true,
        "inputs": [
            {
                "internalType": "address",
                "name": "owner",
                "type": "address"
            },
            {
                "internalType": "address",
                "name": "operator",
                "type": "address"
            }
        ],
        "name": "isApprovedForAll",
        "outputs": [
            {
                "internalType": "bool",
                "name": "",
                "type": "bool"
            }
        ],
        "payable": false,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "internalType": "bytes32",
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "recordExists",
        "outputs": [
            {
                "internalType": "bool",
                "name": "",
                "type": "bool"
            }
        ],
        "payable": false,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "internalType": "bytes32",
                "name": "node",
                "type": "bytes32"
            },
            {
                "internalType": "bytes32",
                "name": "label",
                "type": "bytes32"
            },
            {
                "internalType": "address",
                "name": "owner",
                "type": "address"
            },
            {
                "internalType": "address",
                "name": "resolver",
                "type": "address"
            },
            {
                "internalType": "uint64",
                "name": "ttl",
                "type": "uint64"
            }
        ],
        "name": "setSubnodeRecord",
        "outputs": [],
        "payable": false,
        "stateMutability": "nonpayable",
        "type": "function"
    }
]"#;

const RESOLVER_CONTRACT: &str = r#"[
    {
        "constant": true,
        "inputs": [
            {
                "name": "interfaceID",
                "type": "bytes4"
            }
        ],
        "name": "supportsInterface",
        "outputs": [
            {
                "name": "",
                "type": "bool"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "contentTypes",
                "type": "uint256"
            }
        ],
        "name": "ABI",
        "outputs": [
            {
                "name": "contentType",
                "type": "uint256"
            },
            {
                "name": "data",
                "type": "bytes"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "hash",
                "type": "bytes"
            }
        ],
        "name": "setMultihash",
        "outputs": [],
        "payable": false,
        "stateMutability": "nonpayable",
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "multihash",
        "outputs": [
            {
                "name": "",
                "type": "bytes"
            }
        ],
        "payable": false,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "x",
                "type": "bytes32"
            },
            {
                "name": "y",
                "type": "bytes32"
            }
        ],
        "name": "setPubkey",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "content",
        "outputs": [
            {
                "name": "ret",
                "type": "bytes32"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "addr",
        "outputs": [
            {
                "name": "ret",
                "type": "address"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "contentType",
                "type": "uint256"
            },
            {
                "name": "data",
                "type": "bytes"
            }
        ],
        "name": "setABI",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "name",
        "outputs": [
            {
                "name": "ret",
                "type": "string"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "name",
                "type": "string"
            }
        ],
        "name": "setName",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "hash",
                "type": "bytes32"
            }
        ],
        "name": "setContent",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "pubkey",
        "outputs": [
            {
                "name": "x",
                "type": "bytes32"
            },
            {
                "name": "y",
                "type": "bytes32"
            }
        ],
        "payable": false,
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "addr",
                "type": "address"
            }
        ],
        "name": "setAddr",
        "outputs": [],
        "payable": false,
        "type": "function"
    },
    {
        "inputs": [
            {
                "name": "ensAddr",
                "type": "address"
            }
        ],
        "payable": false,
        "type": "constructor"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "a",
                "type": "address"
            }
        ],
        "name": "AddrChanged",
        "type": "event"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "hash",
                "type": "bytes32"
            }
        ],
        "name": "ContentChanged",
        "type": "event"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "name",
                "type": "string"
            }
        ],
        "name": "NameChanged",
        "type": "event"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": true,
                "name": "contentType",
                "type": "uint256"
            }
        ],
        "name": "ABIChanged",
        "type": "event"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "x",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "y",
                "type": "bytes32"
            }
        ],
        "name": "PubkeyChanged",
        "type": "event"
    },
    {
        "anonymous": false,
        "inputs": [
            {
                "indexed": true,
                "name": "node",
                "type": "bytes32"
            },
            {
                "indexed": false,
                "name": "hash",
                "type": "bytes"
            }
        ],
        "name": "ContenthashChanged",
        "type": "event"
    },
    {
        "constant": true,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            }
        ],
        "name": "contenthash",
        "outputs": [
            {
                "name": "",
                "type": "bytes"
            }
        ],
        "payable": false,
        "stateMutability": "view",
        "type": "function"
    },
    {
        "constant": false,
        "inputs": [
            {
                "name": "node",
                "type": "bytes32"
            },
            {
                "name": "hash",
                "type": "bytes"
            }
        ],
        "name": "setContenthash",
        "outputs": [],
        "payable": false,
        "stateMutability": "nonpayable",
        "type": "function"
    }
]"#;
