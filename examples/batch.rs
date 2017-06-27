extern crate futures_cpupool;
extern crate tokio_core;
extern crate web3;

use std::thread;
use web3::futures::Future;

fn main() {
  let mut event_loop = tokio_core::reactor::Core::new().unwrap();
  let remote = event_loop.remote();

  thread::spawn(move || {
    let pool = futures_cpupool::CpuPool::new(4);
    let http = web3::transports::Http::new("http://localhost:8545").unwrap();

    let web3 = web3::Web3::new(web3::transports::Batch::new(http));
    let _ = web3.eth().accounts();

    let block = web3.eth().block_number().then(|block| {
      println!("Best Block: {:?}", block);
      Ok(())
    });

    let result = web3.transport().submit_batch()
      .then(|accounts| {
        println!("Result: {:?}", accounts);
        Ok(())
      });

    let future = pool.spawn(result);
    remote.spawn(move |_| block);
    remote.spawn(move |_| future);
  });

  loop {
    event_loop.turn(None);
  }
}
