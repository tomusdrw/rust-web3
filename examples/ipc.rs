extern crate tokio_core;
extern crate futures;
extern crate web3;

use tokio_core::reactor;
use futures::Future;

fn main() {
  let mut event_loop = reactor::Core::new().unwrap();
  event_loop.remote().spawn(|handle| {
    let ipc = web3::transports::Ipc::new("./jsonrpc.ipc", handle).unwrap();
    let web3 = web3::Web3::new(ipc);
    println!("Calling accounts.");

    web3.eth().accounts().then(|accounts| {
      println!("Accounts: {:?}", accounts);
      Ok(())
    })
  });

  loop {
    event_loop.turn(None);
  }
}
