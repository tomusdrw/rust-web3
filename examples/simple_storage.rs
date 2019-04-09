//based on examples/contract.rs
extern crate rustc_hex;
extern crate web3;

use rustc_hex::FromHex;
use std::time;
use web3::contract::{Contract, Options};
use web3::futures::Future;
use web3::types::U256;

fn main() {
    let (_eloop, transport) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(transport);
    let accounts = web3.eth().accounts().wait().unwrap();

    //Get current balance
    let balance = web3.eth().balance(accounts[0], None).wait().unwrap();

    println!("Balance: {}", balance);

    // Get the contract bytecode for instance from Solidity compiler
    let bytecode: Vec<u8> = include_str!("./build/SimpleStorage.bin").from_hex().unwrap();
    // Deploying a contract
    let contract = Contract::deploy(web3.eth(), include_bytes!("./build/SimpleStorage.abi"))
        .unwrap()
        .confirmations(0)
        .poll_interval(time::Duration::from_secs(10))
        .options(Options::with(|opt| opt.gas = Some(3_000_000.into())))
        .execute(bytecode, (), accounts[0])
        .unwrap()
        .wait()
        .unwrap();

    println!("{}", contract.address());

    //interact with the contract
    let result = contract.query("get", (), None, Options::default(), None);
    let storage: U256 = result.wait().unwrap();
    println!("{}", storage);

    //Change state of the contract
    contract.call("set", (42,), accounts[0], Options::default());

    //View changes made
    let result = contract.query("get", (), None, Options::default(), None);
    let storage: U256 = result.wait().unwrap();
    println!("{}", storage);
}
