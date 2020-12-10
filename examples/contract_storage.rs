//based on examples/contract.rs

use std::time;
use web3::{
    contract::{Contract, Options},
    types::U256,
};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let _ = env_logger::try_init();
    let transport = web3::transports::Http::new("http://localhost:8545")?;
    let web3 = web3::Web3::new(transport);
    let accounts = web3.eth().accounts().await?;

    // Get current balance
    let balance = web3.eth().balance(accounts[0], None).await?;

    println!("Balance: {}", balance);

    // Get the contract bytecode for instance from Solidity compiler
    let bytecode = include_str!("./res/SimpleStorage.bin");
    // Deploying a contract
    let contract = Contract::deploy(web3.eth(), include_bytes!("./res/SimpleStorage.abi"))?
        .confirmations(1)
        .poll_interval(time::Duration::from_secs(10))
        .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
        .execute(bytecode, (), accounts[0])
        .await?;

    println!("Deployed at: {}", contract.address());

    // interact with the contract
    let result = contract.query("get", (), None, Options::default(), None);
    let storage: U256 = result.await?;
    println!("Get Storage: {}", storage);

    // Change state of the contract
    let tx = contract.call("set", (42_u32,), accounts[0], Options::default()).await?;
    println!("TxHash: {}", tx);

    // consider using `async_std::task::sleep` instead.
    std::thread::sleep(std::time::Duration::from_secs(5));

    // View changes made
    let result = contract.query("get", (), None, Options::default(), None);
    let storage: U256 = result.await?;
    println!("Get again: {}", storage);

    Ok(())
}
