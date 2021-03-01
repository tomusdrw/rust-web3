//! `Eth` namespace, ens

use crate::{
    api::Eth,
    api::Namespace,
    contract::{Contract, Options},
    error::Error,
    helpers::{self, CallFuture},
    signing::namehash,
    types::{
        Address, Block, BlockHeader, BlockId, BlockNumber, Bytes, CallRequest, Filter, Index, Log, SyncState,
        Transaction, TransactionId, TransactionReceipt, TransactionRequest, Work, H256, H520, H64, U256, U64,
    },
    Transport,
};

type ContractError = crate::contract::Error;

use idna::Config;

const ENS_REGISTRY_ADDRESS: &str = "00000000000C2E074eC69A0dFb2997BA6C7d2e1e";

const ADDR_INTERFACE_ID: &str = "3b3b57de";
const NAME_INTERFACE_ID: &str = "691f3431";
const ABI_INTERFACE_ID: &str = "2203ab56";
const PUBKEY_INTERFACE_ID: &str = "c8690233";
const TEXT_INTERFACE_ID: &str = "59d1d43c";
const CONTENTHASH_INTERFACE_ID: &str = "bc1c58d1";

/// `Eth` namespace, ens
#[derive(Debug, Clone)]
pub struct EthEns<T> {
    registry: Registry<T>,
    resolver: Resolver<T>,

    transport: T,
}

impl<T: Transport> Namespace<T> for EthEns<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        let registry = Registry::new(transport.clone());
        let resolver = Resolver::new(transport.clone());

        Self {
            registry,
            resolver,
            transport,
        }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> EthEns<T> {
    pub async fn get_content_hash(&self, domain: &str) -> Result<Bytes, Error> {
        //??? how to choose destination

        let idna = Config::default()
            .transitional_processing(false)
            .use_std3_ascii_rules(true);

        let domain = idna.to_ascii(domain).expect("Cannot Normalize");

        let node = namehash(&domain);

        let resolver_addr = self.registry.get_resolver(Bytes::from(node)).await?;

        let interface_id = Bytes::from([0xbc, 0x1c, 0x58, 0xd1]);
        if !self.resolver.check_interface_support(interface_id).await? {
            //??? error type
            return Err(Error::Unreachable);
        }

        Ok("".into())
    }
}

#[derive(Debug, Clone)]
struct Registry<T: Transport> {
    contract: Contract<T>,
}

impl<T: Transport> Registry<T> {
    fn new(eth: Eth<T>) -> Self {
        let address = ENS_REGISTRY_ADDRESS.parse().expect("Parsing Address Failed");

        let contract = Contract::from_json(eth, address, REGISTRY_CONTRACT.as_bytes()).expect("Contract Creation Failed")

        Self {
            contract
        }
    }
}

impl<T: Transport> Registry<T> {
    //https://github.com/ensdomains/ens/blob/master/contracts/ENS.sol

    // https://docs.ens.domains/contract-api-reference/ens#set-record
    async fn set_record(&self, node: Bytes, owner: Address, resolver: Address, ttl: u64) -> Result<bool, ContractError> {
        let options = Options::default();
        

        self.contract.call("setRecord", (node, owner, resolver, ttl), None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-subdomain-record
    async fn set_subnode_record(
        &self,
        node: Bytes,
        label: Bytes,
        owner: Address,
        resolver: Address,
    ) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let label = helpers::serialize(&label);
        let owner = helpers::serialize(&owner);
        let resolver = helpers::serialize(&resolver);
        CallFuture::new(
            self.transport
                .execute("ens_setSubnodeRecord", vec![node, label, owner, resolver]),
        )
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-subdomain-owner
    async fn set_subnode_owner(&self, node: Bytes, label: Bytes, owner: Address) -> CallFuture<Bytes, T::Out> {
        let node = helpers::serialize(&node);
        let label = helpers::serialize(&label);
        let owner = helpers::serialize(&owner);
        CallFuture::new(self.transport.execute("ens_setSubnodeOwner", vec![node, label, owner]))
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-resolver
    async fn set_resolver(&self, node: Bytes, resolver: Address) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let resolver = helpers::serialize(&resolver);
        CallFuture::new(self.transport.execute("ens_setResolver", vec![node, resolver]))
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-owner
    async fn set_owner(&self, node: Bytes, owner: Address) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let owner = helpers::serialize(&owner);
        CallFuture::new(self.transport.execute("ens_setOwner", vec![node, owner]))
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-ttl
    async fn set_ttl(&self, node: Bytes, ttl: U64) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let ttl = helpers::serialize(&ttl);
        CallFuture::new(self.transport.execute("ens_setTTL", vec![node, ttl]))
    }

    // https://docs.ens.domains/contract-api-reference/ens#set-approval
    async fn set_approval_for_all(&self, operator: Address, approved: bool) -> CallFuture<bool, T::Out> {
        let operator = helpers::serialize(&operator);
        let approved = helpers::serialize(&approved);
        CallFuture::new(
            self.transport
                .execute("ens_setApprovalForAll", vec![operator, approved]),
        )
    }

    // https://docs.ens.domains/contract-api-reference/ens#get-owner
    async fn get_owner(&self, node: Bytes) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("owner", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#get-resolver
    async fn get_resolver(&self, node: Bytes) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("resolver", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#get-ttl
    async fn get_ttl(&self, node: Bytes) -> Result<u64, ContractError> {
        let options = Options::default();

        self.contract.query("ttl", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#check-record-existence
    async fn check_record_existence(&self, node: Bytes) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract.query("recordExists", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/ens#check-approval
    async fn check_approval(&self, owner: Address, operator: Address) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract.query("isApprovedForAll", (owner, operator), None, options, None).await
    }
}

#[derive(Debug, Clone)]
struct Resolver<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for Resolver<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        Self { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Resolver<T> {
    // https://github.com/ensdomains/resolvers/blob/master/contracts/Resolver.sol

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-contract-abi
    fn get_abi(&self, node: Bytes, content_types: U256) -> CallFuture<(U256, Bytes), T::Out> {
        let node = helpers::serialize(&node);
        let content_types = helpers::serialize(&content_types);
        CallFuture::new(self.transport.execute("ens_ABI", vec![node, content_types]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-ethereum-address
    fn get_ethereum_address(&self, node: Bytes) -> CallFuture<Address, T::Out> {
        let node = helpers::serialize(&node);
        CallFuture::new(self.transport.execute("ens_addr", vec![node]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-blockchain-address
    fn get_blockchain_address(&self, node: Bytes, coin_type: U256) -> CallFuture<Bytes, T::Out> {
        let node = helpers::serialize(&node);
        let coin_type = helpers::serialize(&coin_type);
        CallFuture::new(self.transport.execute("ens_addr", vec![node, coin_type]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-content-hash
    fn get_content_hash(&self, node: Bytes) -> CallFuture<Bytes, T::Out> {
        let node = helpers::serialize(&node);
        CallFuture::new(self.transport.execute("ens_contenthash", vec![node]))
    }

    //dnsrr???

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-canonical-name
    fn get_canonical_name(&self, node: Bytes) -> CallFuture<String, T::Out> {
        let node = helpers::serialize(&node);
        CallFuture::new(self.transport.execute("ens_name", vec![node]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-public-key
    fn get_public_key(&self, node: Bytes) -> CallFuture<(Bytes, Bytes), T::Out> {
        let node = helpers::serialize(&node);
        CallFuture::new(self.transport.execute("ens_pubkey", vec![node]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-text-data
    fn get_text_data(&self, node: Bytes, key: String) -> CallFuture<String, T::Out> {
        //??? not sure what type string should be
        let node = helpers::serialize(&node);
        let key = helpers::serialize(&key);
        CallFuture::new(self.transport.execute("ens_text", vec![node, key]))
    }

    //interfaceImplementer

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-contract-abi
    fn set_contract_abi(&self, node: Bytes, content_type: U256, data: Bytes) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let content_type = helpers::serialize(&content_type);
        let data = helpers::serialize(&data);
        CallFuture::new(self.transport.execute("ens_setABI", vec![node, content_type, data]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-ethereum-address
    fn set_ethereum_address(&self, node: Bytes, address: Address) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let address = helpers::serialize(&address);
        CallFuture::new(self.transport.execute("ens_setAddr", vec![node, address]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-blockchain-address
    fn set_blockchain_address(&self, node: Bytes, coin_type: U256, a: Bytes) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let coin_type = helpers::serialize(&coin_type);
        let a = helpers::serialize(&a);
        CallFuture::new(self.transport.execute("ens_setAddr", vec![node, coin_type, a]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-content-hash
    fn set_content_hash(&self, node: Bytes, hash: Bytes) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let hash = helpers::serialize(&hash);
        CallFuture::new(self.transport.execute("ens_setContenthash", vec![node, hash]))
    }

    //setDnsrr

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-canonical-name
    fn set_canonical_name(&self, node: Bytes, name: String) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let name = helpers::serialize(&name);
        CallFuture::new(self.transport.execute("ens_setName", vec![node, name]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-public-key
    fn set_public_key(&self, node: Bytes, x: Bytes, y: Bytes) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let x = helpers::serialize(&x);
        let y = helpers::serialize(&y);
        CallFuture::new(self.transport.execute("ens_setPubkey", vec![node, x, y]))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-text-data
    fn set_text_data(&self, node: Bytes, key: String, value: String) -> CallFuture<bool, T::Out> {
        let node = helpers::serialize(&node);
        let key = helpers::serialize(&key);
        let value = helpers::serialize(&value);
        CallFuture::new(self.transport.execute("ens_setText", vec![node, key, value]))
    }

    //setInterface

    // https://docs.ens.domains/contract-api-reference/publicresolver#check-interface-support
    fn check_interface_support(&self, interface_id: Bytes) -> CallFuture<bool, T::Out> {
        let interface_id = helpers::serialize(&interface_id);
        CallFuture::new(self.transport.execute("ens_supportsInterface", vec![interface_id]))
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