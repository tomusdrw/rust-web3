use hex_literal::hex;
use web3::{
    api::Namespace,
    contract::ens::Ens,
    contract::{Contract, Options},
    types::Address,
};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let _ = env_logger::try_init();
    let http = web3::transports::Http::new("http://localhost:8545")?;
    let web3 = web3::Web3::new(http);

    let my_account = hex!("d028d24f16a8893bd078259d413372ac01580769").into();

    let bytecode = include_str!("./res/ENSRegistry.code").trim_end();
    let data: [u8; 20] = hex!("314159265dd8dbb310642f98f50c066173c1259b");
    let addr: Address = data.into();

    // Deploying registry
    let registry_contract = Contract::deploy(web3.eth(), include_bytes!("../src/contract/ens/ENSRegistry.json"))?
        .confirmations(0)
        .options(Options::with(|opt| {
            opt.value = Some(0.into());
            opt.gas_price = Some(5.into());
            opt.gas = Some(3_000_000.into());
        }))
        .execute(bytecode, addr, my_account)
        .await?;

    assert_eq!(
        registry_contract.address(),
        hex!("00000000000C2E074eC69A0dFb2997BA6C7d2e1e").into()
    );

    let bytecode = include_str!("./res/PublicResolver.code").trim_end();
    let data: [u8; 20] = hex!("314159265dd8dbb310642f98f50c066173c1259b");
    let addr: Address = data.into();

    // Deploying public resolver
    let resolver_contract = Contract::deploy(web3.eth(), include_bytes!("../src/contract/ens/PublicResolver.json"))?
        .confirmations(0)
        .options(Options::with(|opt| {
            opt.value = Some(0.into());
            opt.gas_price = Some(5.into());
            opt.gas = Some(3_000_000.into());
        }))
        .execute(bytecode, addr, my_account)
        .await?;

    Ok(())
}
