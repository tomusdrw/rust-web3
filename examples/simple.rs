extern crate web3;

use web3::futures::Future;

fn main() {
  let (_eloop, http) = web3::transports::Http::new("http://localhost:8545").unwrap();
  let web3 = web3::Web3::new(http);
  let accounts = web3.eth().accounts().wait().unwrap();

  println!("Accounts: {:?}", accounts);
}
