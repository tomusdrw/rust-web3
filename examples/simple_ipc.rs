extern crate futures;
extern crate web3;

use futures::Future;

fn main() {
  let (_el, transport) = web3::transports::Ipc::with_event_loop("./jsonrpc.ipc").unwrap();
  let web3 = web3::Web3::new(transport);

  println!("Calling accounts.");
  let accounts = web3.eth().accounts().wait().unwrap();
  println!("Accounts: {:?}", accounts);
}
