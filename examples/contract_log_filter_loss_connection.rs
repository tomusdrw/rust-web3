use std::time::Duration;

use ethabi::Address;
use futures::{future, TryStreamExt};
use hex_literal::hex;
use web3::{
    api::BaseFilter,
    contract::{Contract, Options},
    transports::WebSocket,
    types::{Filter, FilterBuilder, Log},
    Web3,
};

#[tokio::main]
async fn main() -> web3::contract::Result<()> {
    let _ = env_logger::try_init();
    let transport = web3::transports::WebSocket::new("ws://localhost:8545").await?;
    let web3 = web3::Web3::new(transport);

    println!("Calling accounts.");
    let accounts = web3.eth().accounts().await?;

    let bytecode = include_str!("./res/SimpleEvent.bin");
    let contract = Contract::deploy(web3.eth(), include_bytes!("./res/SimpleEvent.abi"))?
        .confirmations(1)
        .poll_interval(Duration::from_secs(10))
        .options(Options::with(|opt| opt.gas = Some(3_000_000u64.into())))
        .execute(bytecode, (), accounts[0])
        .await
        .unwrap();

    println!("contract deployed at: {}", contract.address());

    tokio::spawn(interval_contract_call(contract.clone(), accounts[0]));

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

    loop {
        let filter = get_filter(web3.clone(), &filter).await;
        let logs_stream = filter.stream(Duration::from_secs(2));
        let res = logs_stream
            .try_for_each(|log| {
                println!("Get log: {:?}", log);
                future::ready(Ok(()))
            })
            .await;

        if let Err(e) = res {
            println!("Log Filter Error: {}", e);
        }
    }
}

async fn interval_contract_call(contract: Contract<WebSocket>, account: Address) {
    loop {
        match contract.call("hello", (), account, Options::default()).await {
            Ok(tx) => println!("got tx: {:?}", tx),
            Err(e) => println!("get tx failed: {}", e),
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

pub async fn get_filter(web3: Web3<WebSocket>, filter: &Filter) -> BaseFilter<WebSocket, Log> {
    loop {
        match web3.eth_filter().create_logs_filter(filter.clone()).await {
            Err(e) => println!("get filter failed: {}", e),
            Ok(filter) => return filter,
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}
