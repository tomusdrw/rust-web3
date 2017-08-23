extern crate parking_lot;
extern crate web3;

use std::{time, thread};
use std::sync::{atomic, Arc};
use parking_lot::Mutex;
use web3::futures::Future;


fn as_millis(dur: time::Duration) -> u64 {
  dur.as_secs() * 1_000 + dur.subsec_nanos() as u64 / 1_000_000
}

struct Ticker {
  id: String,
  started: atomic::AtomicUsize,
  reqs: atomic::AtomicUsize,
  time: Mutex<time::Instant>,
}

impl Ticker {
  pub fn new(id: &str) -> Self {
    Ticker {
      id: id.to_owned(),
      time: Mutex::new(time::Instant::now()),
      started: Default::default(),
      reqs: Default::default(),
    }
  }

  pub fn start(&self) {
    self.started.fetch_add(1, atomic::Ordering::AcqRel);
  }

  pub fn tick(&self) {
    let reqs = self.reqs.fetch_add(1, atomic::Ordering::AcqRel) as u64;
    self.started.fetch_sub(1, atomic::Ordering::AcqRel);

    if reqs >= 100_000 {
      self.print_result(reqs);
    }
  }

  pub fn print_result(&self, reqs: u64) {
    let mut time = self.time.lock();
    let elapsed = as_millis(time.elapsed());
    let result = reqs * 1_000 / elapsed;

    println!("[{}] {} reqs/s ({} reqs in {} ms)", self.id, result, reqs, elapsed);

    self.reqs.store(0, atomic::Ordering::Release);
    *time = time::Instant::now();
  }

  pub fn wait(&self) {
    while self.started.load(atomic::Ordering::Relaxed) > 0 {
      thread::sleep(time::Duration::from_millis(100));
    }
    self.print_result(self.reqs.load(atomic::Ordering::Acquire) as u64);
  }
}


fn main() {
  let requests = 200_000;
  let (eloop, http) = web3::transports::Http::new("http://localhost:8545/").unwrap();
  bench("http", eloop, http, requests);

  let (eloop, http) = web3::transports::Ipc::new("/home/tomusdrw/.local/share/io.parity.ethereum/jsonrpc.ipc").unwrap();
  bench(" ipc", eloop, http, requests);
}

fn bench<T: web3::Transport>(id: &str, eloop: web3::transports::EventLoopHandle, transport: T, max: usize) where
  T::Out: Send + 'static,
{
  let web3 = web3::Web3::new(transport);
  let ticker = Arc::new(Ticker::new(id));
  for _ in 0..max {
    let ticker = ticker.clone();
    ticker.start();
    let accounts = web3.eth().block_number().then(move |res| {
      if let Err(e) = res {
        println!("Error: {:?}", e);
      }
      ticker.tick();
      Ok(())
    });
    eloop.remote().spawn(|_| accounts);
  }

  ticker.wait()
}
