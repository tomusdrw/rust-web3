use crate::{
    api::Eth,
    contract::{Contract, Options},
    signing::NameHash,
    types::{Address, TransactionId},
    Transport,
};

type ContractError = crate::contract::Error;

// See https://github.com/ensdomains/resolvers/blob/master/contracts/DefaultReverseResolver.sol for contract interface.
#[derive(Debug, Clone)]
pub struct ReverseResolver<T: Transport> {
    contract: Contract<T>,
}

impl<T: Transport> ReverseResolver<T> {
    pub fn new(eth: Eth<T>, resolver_addr: Address) -> Self {
        // See https://github.com/ensdomains/ens-contracts for up to date contracts.
        let bytes = include_bytes!("DefaultReverseResolver.json");

        let contract = Contract::from_json(eth, resolver_addr, bytes).expect("Contract Creation Failed");

        Self { contract }
    }
}

impl<T: Transport> ReverseResolver<T> {
    pub async fn canonical_name(&self, node: NameHash) -> Result<String, ContractError> {
        let options = Options::default();

        self.contract.query("name", node, None, options, None).await
    }

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
