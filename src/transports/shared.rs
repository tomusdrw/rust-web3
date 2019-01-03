use std::{fmt, mem, thread};
use std::sync::{self, atomic, Arc};
use futures::{self, Future};
use futures::sync::oneshot;
use transports::Result;
use transports::tokio_core::reactor;
use {Error, ErrorKind, RequestId};

/// Event Loop Handle.
/// NOTE: Event loop is stopped when handle is dropped!
#[derive(Debug)]
pub struct EventLoopHandle {
    thread: Option<thread::JoinHandle<()>>,
    remote: reactor::Remote,
    done: Arc<atomic::AtomicBool>,
}

impl EventLoopHandle {
    /// Creates a new `EventLoopHandle` and transport given the transport initializer.
    pub fn spawn<T, F>(func: F) -> Result<(Self, T)>
    where
        F: FnOnce(&reactor::Handle) -> Result<T>,
        F: Send + 'static,
        T: Send + 'static,
    {
        let done = Arc::new(atomic::AtomicBool::new(false));
        let (tx, rx) = sync::mpsc::sync_channel(1);
        let done2 = done.clone();

        let eloop = thread::spawn(move || {
            let run = move || {
                let event_loop = reactor::Core::new()?;
                let http = func(&event_loop.handle())?;
                Ok((http, event_loop))
            };

            let send = move |result| {
                tx.send(result).expect("Receiving end is always waiting.");
            };

            let res = run();
            match res {
                Err(e) => send(Err(e)),
                Ok((http, mut event_loop)) => {
                    send(Ok((http, event_loop.remote())));

                    while !done2.load(atomic::Ordering::Relaxed) {
                        event_loop.turn(None);
                    }
                }
            }
        });

        rx.recv()
            .expect("Thread is always spawned.")
            .map(|(http, remote)| {
                (
                    EventLoopHandle {
                        thread: Some(eloop),
                        remote,
                        done,
                    },
                    http,
                )
            })
    }

    /// Returns event loop remote.
    pub fn remote(&self) -> &reactor::Remote {
        &self.remote
    }
}

impl Drop for EventLoopHandle {
    fn drop(&mut self) {
        self.done.store(true, atomic::Ordering::Relaxed);
        self.remote.spawn(|_| Ok(()));
        self.thread
            .take()
            .expect("We never touch thread except for drop; drop happens only once; qed")
            .join()
            .expect("Thread should shut down cleanly.");
    }
}

type PendingResult<O> = oneshot::Receiver<Result<O>>;

enum RequestState<O> {
    Sending(Option<Result<()>>, PendingResult<O>),
    WaitingForResponse(PendingResult<O>),
    Done,
}

/// A future representing a response to a pending request.
pub struct Response<T, O> {
    id: RequestId,
    state: RequestState<O>,
    extract: T,
}

impl<T, O> Response<T, O> {
    /// Creates a new `Response`
    pub fn new(id: RequestId, result: Result<()>, rx: PendingResult<O>, extract: T) -> Self {
        Response {
            id,
            extract,
            state: RequestState::Sending(Some(result), rx),
        }
    }
}

impl<T, O, Out> Future for Response<T, O>
where
    T: Fn(O) -> Result<Out>,
    Out: fmt::Debug,
{
    type Item = Out;
    type Error = Error;

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        loop {
            let extract = &self.extract;
            match self.state {
                RequestState::Sending(ref mut result, _) => {
                    trace!("[{}] Request pending.", self.id);
                    if let Some(Err(e)) = result.take() {
                        return Err(e);
                    }
                }
                RequestState::WaitingForResponse(ref mut rx) => {
                    trace!("[{}] Checking response.", self.id);
                    let result = try_ready!(
                        rx.poll()
                            .map_err(|_| Error::from(ErrorKind::Io(::std::io::ErrorKind::TimedOut.into())))
                    );
                    trace!("[{}] Extracting result.", self.id);
                    return result.and_then(|x| extract(x)).map(futures::Async::Ready);
                }
                RequestState::Done => {
                    return Err(ErrorKind::Unreachable.into());
                }
            }
            // Proceeed to the next state
            let state = mem::replace(&mut self.state, RequestState::Done);
            self.state = if let RequestState::Sending(_, rx) = state {
                RequestState::WaitingForResponse(rx)
            } else {
                state
            }
        }
    }
}
