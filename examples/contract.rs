extern crate rustc_hex;
extern crate web3;

use web3::futures::Future;
use web3::contract::{Contract, Options};
use web3::types::{Address, U256};
use rustc_hex::FromHex;

fn main() {
    let (_eloop, http) = web3::transports::Http::new("http://localhost:8545").unwrap();
    let web3 = web3::Web3::new(http);

    let my_account: Address = "00a329c0648769a73afac7f9381e08fb43dbea72"
        .parse()
        .unwrap();
    // Get the contract bytecode for instance from Solidity compiler
    let bytecode: Vec<u8> = include_str!("./contract_token.code").from_hex().unwrap();
    // Deploying a contract
    let contract = Contract::deploy(web3.eth(), include_bytes!("../src/contract/res/token.json"))
        .unwrap()
        .confirmations(4)
        .options(Options::with(|opt| {
            opt.value = Some(5.into())
        }))
        .execute(
            bytecode,
            (
                U256::from(1_000_000),
                "My Token".to_owned(),
                3u64,
                "MT".to_owned(),
            ),
            my_account,
        )
        .expect("Correct parameters are passed to the constructor.")
        .wait()
        .unwrap();

    let result = contract.query("balanceOf", (my_account,), None, Options::default(), None);
    let balance_of: U256 = result.wait().unwrap();
    assert_eq!(balance_of, 1_000_000.into());

    // Accessing existing contract
    let contract_address = contract.address();
    let contract = Contract::from_json(
        web3.eth(),
        contract_address,
        include_bytes!("../src/contract/res/token.json"),
    ).unwrap();

    let result = contract.query("balanceOf", (my_account,), None, Options::default(), None);
    let balance_of: U256 = result.wait().unwrap();
    assert_eq!(balance_of, 1_000_000.into());
}
