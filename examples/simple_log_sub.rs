extern crate rustc_hex;
extern crate tokio_core;
extern crate web3;

use std::time;
use web3::contract::{Contract, Options};
use web3::futures::{Future, Stream};
use web3::types::FilterBuilder;

fn main() {
    let mut eloop = tokio_core::reactor::Core::new().unwrap();
    let web3 =
        web3::Web3::new(web3::transports::WebSocket::with_event_loop("ws://localhost:8546", &eloop.handle()).unwrap());

    // Get the contract bytecode for instance from Solidity compiler
    let bytecode = include_str!("./build/SimpleEvent.bin");

    eloop
        .run(web3.eth().accounts().then(|accounts| {
            let accounts = accounts.unwrap();
            println!("accounts: {:?}", &accounts);

            Contract::deploy(web3.eth(), include_bytes!("./build/SimpleEvent.abi"))
                .unwrap()
                .confirmations(1)
                .poll_interval(time::Duration::from_secs(10))
                .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
                .execute(bytecode, (), accounts[0])
                .unwrap()
                .then(move |contract| {
                    let contract = contract.unwrap();
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

                    let event_future = web3
                        .eth_subscribe()
                        .subscribe_logs(filter)
                        .then(|sub| {
                            sub.unwrap().for_each(|log| {
                                println!("got log: {:?}", log);
                                Ok(())
                            })
                        })
                        .map_err(|_| ());

                    let call_future = contract.call("hello", (), accounts[0], Options::default()).then(|tx| {
                        println!("got tx: {:?}", tx);
                        Ok(())
                    });

                    event_future.join(call_future)
                })
        }))
        .unwrap();
}
