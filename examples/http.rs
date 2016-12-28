extern crate futures;
extern crate futures_cpupool;
extern crate tokio_core;
extern crate web3;

use std::thread;
use futures::Future;

fn main() {
  let mut event_loop = tokio_core::reactor::Core::new().unwrap();
  let remote = event_loop.remote();

  thread::spawn(move || {
    let pool = futures_cpupool::CpuPool::new(4);

    let web3 = web3::Web3::new(web3::transports::Http::new("http://localhost:8545").unwrap());
    let accounts = web3.eth().accounts().then(|accounts| {
      println!("Accounts: {:?}", accounts);
      Ok(())
    });

    let future = pool.spawn(accounts);
    remote.spawn(move |_| future);
  });

  loop {
    event_loop.turn(None);
  }
}
