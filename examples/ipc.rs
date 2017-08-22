extern crate tokio_core;
extern crate web3;

use tokio_core::reactor;
use web3::futures::Future;

fn main() {
  let mut event_loop = reactor::Core::new().unwrap();
  let handle = event_loop.handle();

  let ipc = web3::transports::Ipc::with_event_loop("./jsonrpc.ipc", &handle).unwrap();
  let web3 = web3::Web3::new(ipc);
  println!("Calling accounts.");

  let future = web3.eth().accounts().map(|accounts| {
    println!("Accounts: {:?}", accounts);
  });

  event_loop.run(future).unwrap();
}
