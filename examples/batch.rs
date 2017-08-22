extern crate tokio_core;
extern crate web3;

use web3::futures::Future;

const MAX_PARALLEL_REQUESTS: usize = 64;

fn main() {
  let mut event_loop = tokio_core::reactor::Core::new().unwrap();
  let remote = event_loop.remote();

  let http = web3::transports::Http::with_event_loop(
    "http://localhost:8545",
    &event_loop.handle(),
    MAX_PARALLEL_REQUESTS,
  ).unwrap();

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

  remote.spawn(move |_| block);
  remote.spawn(move |_| result);

  loop {
    event_loop.turn(None);
  }
}
