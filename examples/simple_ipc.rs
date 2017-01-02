extern crate env_logger;
extern crate futures;
extern crate web3;

use futures::Future;

fn main() {
  env_logger::init().unwrap();

  let (_el, transport) = web3::transports::Ipc::with_event_loop("./jsonrpc.ipc").unwrap();
  let web3 = web3::Web3::new(transport);

  println!("Calling accounts.");
  let accounts = web3.eth().accounts().wait().unwrap();
  println!("Accounts: {:?}", accounts);

  println!("Calling balance.");
  let balance = web3.eth().balance(0.into(), None).wait().unwrap();
  println!("Balance: {}", balance);
}
