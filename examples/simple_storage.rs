//based on examples/contract.rs

use std::time;
use web3::contract::{Contract, Options};
use web3::types::U256;

fn main() -> web3::contract::Result<()> {
    web3::block_on(run())
}

async fn run() -> web3::contract::Result<()> {
    let transport = web3::transports::Http::new("http://localhost:8545")?;
    let web3 = web3::Web3::new(transport);
    let accounts = web3.eth().accounts().await?;

    //Get current balance
    let balance = web3.eth().balance(accounts[0], None).await?;

    println!("Balance: {}", balance);

    // Get the contract bytecode for instance from Solidity compiler
    let bytecode = include_str!("./build/SimpleStorage.bin");
    // Deploying a contract
    let contract = Contract::deploy(web3.eth(), include_bytes!("./build/SimpleStorage.abi"))?
        .confirmations(0)
        .poll_interval(time::Duration::from_secs(10))
        .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
        .execute(bytecode, (), accounts[0])?
        .await?;

    println!("{}", contract.address());

    //interact with the contract
    let result = contract.query("get", (), None, Options::default(), None);
    let storage: U256 = result.await?;
    println!("{}", storage);

    //Change state of the contract
    contract.call("set", (42_u32,), accounts[0], Options::default()).await?;

    //View changes made
    let result = contract.query("get", (), None, Options::default(), None);
    let storage: U256 = result.await?;
    println!("{}", storage);

    Ok(())
}
