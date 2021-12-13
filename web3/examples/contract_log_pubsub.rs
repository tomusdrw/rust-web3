use hex_literal::hex;
use std::time;
use web3::{
    contract::{Contract, Options},
    futures::{future, StreamExt},
    types::FilterBuilder,
};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let _ = env_logger::try_init();
    let web3 = web3::Web3::new(web3::transports::WebSocket::new("ws://localhost:8546").await?);

    // Get the contract bytecode for instance from Solidity compiler
    let bytecode = include_str!("./res/SimpleEvent.bin");

    let accounts = web3.eth().accounts().await?;
    println!("accounts: {:?}", &accounts);

    let contract = Contract::deploy(web3.eth(), include_bytes!("./res/SimpleEvent.abi"))?
        .confirmations(1)
        .poll_interval(time::Duration::from_secs(10))
        .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
        .execute(bytecode, (), accounts[0]);
    let contract = contract.await?;
    println!("contract deployed at: {}", contract.address());

    // Filter for Hello event in our contract
    let filter = FilterBuilder::default()
        .address(vec![contract.address()])
        .topics(
            Some(vec![hex!(
                "d282f389399565f3671145f5916e51652b60eee8e5c759293a2f5771b8ddfd2e"
            )
            .into()]),
            None,
            None,
            None,
        )
        .build();

    let sub = web3.eth_subscribe().subscribe_logs(filter).await?;

    let tx = contract.call("hello", (), accounts[0], Options::default()).await?;
    println!("got tx: {:?}", tx);

    sub.for_each(|log| {
        println!("got log: {:?}", log);
        future::ready(())
    })
    .await;

    Ok(())
}
