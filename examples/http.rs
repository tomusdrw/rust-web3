extern crate tokio_core;
extern crate web3;

use web3::futures::Future;

fn main() {
  let mut event_loop = tokio_core::reactor::Core::new().unwrap();

  let web3 = web3::Web3::new(web3::transports::Http::with_event_loop(
    "http://localhost:8545",
    &event_loop.handle(),
  ).unwrap());
  let accounts = web3.eth().accounts().map(|accounts| {
    println!("Accounts: {:?}", accounts);
  });

  event_loop.run(accounts).unwrap();
}
