use crate::{
    api::Eth,
    contract::{Contract, Options},
    signing::NameHash,
    types::{Address, Bytes, TransactionId, U256},
    Transport,
};

type ContractError = crate::contract::Error;

// See https://github.com/ensdomains/resolvers/blob/master/contracts/Resolver.sol for resolver interface.
#[derive(Debug, Clone)]
pub struct PublicResolver<T: Transport> {
    contract: Contract<T>,
}

impl<T: Transport> PublicResolver<T> {
    pub fn new(eth: Eth<T>, resolver_addr: Address) -> Self {
        // See https://github.com/ensdomains/ens-contracts for up to date contracts.
        let bytes = include_bytes!("PublicResolver.json");

        let contract = Contract::from_json(eth, resolver_addr, bytes).expect("Contract Creation");

        Self { contract }
    }
}

impl<T: Transport> PublicResolver<T> {
    // https://docs.ens.domains/contract-api-reference/publicresolver#get-contract-abi
    pub async fn _abi(&self, node: NameHash, content_types: U256) -> Result<(U256, Vec<u8>), ContractError> {
        let options = Options::default();

        self.contract
            .query("ABI", (node, content_types), None, options, None)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-ethereum-address
    pub async fn ethereum_address(&self, node: NameHash) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract.query("addr", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-blockchain-address
    pub async fn blockchain_address(&self, node: NameHash, coin_type: U256) -> Result<Vec<u8>, ContractError> {
        let options = Options::default();

        self.contract
            .query("addr", (node, coin_type), None, options, None)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-content-hash
    pub async fn content_hash(&self, node: NameHash) -> Result<Vec<u8>, ContractError> {
        let options = Options::default();

        self.contract.query("contenthash", node, None, options, None).await
    }

    // This function is not explained anywhere. More info needed!
    pub async fn _ddsrr(&self, node: NameHash) -> Result<Bytes, ContractError> {
        let options = Options::default();

        self.contract.query("dnsrr", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-canonical-name
    // A reverse resolver is used by default and so this fucntion is not used.
    pub async fn _canonical_name(&self, node: NameHash) -> Result<String, ContractError> {
        let options = Options::default();

        self.contract.query("name", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-public-key
    pub async fn public_key(&self, node: NameHash) -> Result<([u8; 32], [u8; 32]), ContractError> {
        let options = Options::default();

        self.contract.query("pubkey", node, None, options, None).await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#get-text-data
    pub async fn text_data(&self, node: NameHash, key: String) -> Result<String, ContractError> {
        let options = Options::default();

        self.contract.query("text", (node, key), None, options, None).await
    }

    // This function is not explained anywhere. More info needed!
    pub async fn _interface_implementer(&self, node: NameHash, interface: [u8; 4]) -> Result<Address, ContractError> {
        let options = Options::default();

        self.contract
            .query("interfaceImplementer", (node, interface), None, options, None)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-contract-abi
    pub async fn _set_contract_abi(
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

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-ethereum-address
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

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-blockchain-address
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

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-content-hash
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

    // This function is not explained anywhere. More info needed!
    pub async fn _set_dnsrr(
        &self,
        from: Address,
        node: NameHash,
        data: Vec<u8>,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setDnsrr", (node, data), from, options).await?;

        Ok(TransactionId::Hash(id))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-canonical-name
    // A reverse resolver is used by default and so this fucntion is not used.
    pub async fn _set_canonical_name(
        &self,
        from: Address,
        node: NameHash,
        name: String,
    ) -> Result<TransactionId, ContractError> {
        let options = Options::default();

        let id = self.contract.call("setName", (node, name), from, options).await?;

        Ok(TransactionId::Hash(id))
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-public-key
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

    // https://docs.ens.domains/contract-api-reference/publicresolver#set-text-data
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

    // This function is not explained anywhere. More info needed!
    pub async fn _set_interface(
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

    // https://docs.ens.domains/contract-api-reference/publicresolver#check-interface-support
    pub async fn check_interface_support(&self, interface_id: [u8; 4]) -> Result<bool, ContractError> {
        let options = Options::default();

        self.contract
            .query("supportsInterface", interface_id, None, options, None)
            .await
    }

    // https://docs.ens.domains/contract-api-reference/publicresolver#multicall
    pub async fn _multicall(&self, data: Bytes) -> Result<Bytes, ContractError> {
        let options = Options::default();

        self.contract.query("multicall", data, None, options, None).await
    }
}
