
use std::time;
use web3::contract::{Contract, Options};
use web3::futures::{future, StreamExt};
use web3::types::FilterBuilder;

fn main() -> web3::contract::Result<()> {
    web3::block_on(run())
}

async fn run() -> web3::contract::Result<()> {
    let web3 = web3::Web3::new(web3::transports::Http::new("http://localhost:8545")?);

    // Get the contract bytecode for instance from Solidity compiler
    let bytecode = include_str!("./build/SimpleEvent.bin");

    let accounts = web3.eth().accounts().await?;
    println!("accounts: {:?}", &accounts);

    let contract = Contract::deploy(web3.eth(), include_bytes!("./build/SimpleEvent.abi"))?
        .confirmations(1)
        .poll_interval(time::Duration::from_secs(10))
        .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
        .execute(bytecode, (), accounts[0])?
        .await?;

    println!("contract deployed at: {}", contract.address());

    // Filter for Hello event in our contract
    let filter = FilterBuilder::default()
        .address(vec![contract.address()])
        .topics(
            Some(vec![
                "0xd282f389399565f3671145f5916e51652b60eee8e5c759293a2f5771b8ddfd2e"
                .parse()
                .unwrap(),
            ]),
            None,
            None,
            None,
        )
        .build();

    let filter = web3
        .eth_filter()
        .create_logs_filter(filter)
        .await?;

    filter.stream(time::Duration::from_secs(0))
        .for_each(|log| {
            println!("got log: {:?}", log);
            future::ready(())
        })
        .await;

    let tx = contract.call("hello", (), accounts[0], Options::default()).await?;
    println!("got tx: {:?}", tx);

    Ok(())
}
