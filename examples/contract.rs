extern crate web3;

use web3::futures::Future;
use web3::contract::{Contract, Options};
use web3::types::{Address, U256};

fn main() {
  let (_eloop, http) = web3::transports::Http::new("http://localhost:8545").unwrap();
  let web3 = web3::Web3::new(http);
  let contract_address = 5.into();
  let contract = Contract::from_json(
    web3.eth(),
    contract_address,
    include_bytes!("../src/contract/res/token.json")
  ).unwrap();

  let result = contract.query("balanceOf", (Address::from(10), ), None, Options::default());
  let balance_of: U256 = result.wait().unwrap();
  assert_eq!(balance_of, 10.into());
}
