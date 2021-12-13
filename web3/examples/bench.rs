use parking_lot::Mutex;
use std::{
    sync::{atomic, Arc},
    thread, time,
};

#[tokio::main]
async fn main() -> web3::Result {
    let _ = env_logger::try_init();
    let requests = 200_000;

    let http = web3::transports::Http::new("http://localhost:8545/")?;
    bench("http", http, requests);

    let ipc = web3::transports::WebSocket::new("./jsonrpc.ipc").await?;
    bench(" ipc", ipc, requests);

    Ok(())
}

fn bench<T: web3::Transport>(id: &str, transport: T, max: usize)
where
    T::Out: Send + 'static,
{
    use futures::FutureExt;

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
            futures::future::ready(())
        });
        tokio::spawn(accounts);
    }

    ticker.wait();
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
        fn as_millis(dur: time::Duration) -> u64 {
            dur.as_secs() * 1_000 + dur.subsec_nanos() as u64 / 1_000_000
        }

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
