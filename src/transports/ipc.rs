//! IPC Transport for *nix

extern crate tokio_core;
extern crate tokio_uds;

use std::thread;
use std::sync::{self, atomic, Arc};
use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::Path;

use self::tokio_core::reactor;
use self::tokio_core::io::{ReadHalf, WriteHalf, Io};
use self::tokio_uds::UnixStream;

use futures::{self, Sink, Stream, Future, BoxFuture};
use futures::sync::{oneshot, mpsc};
use helpers;
use rpc::{self, Value as RpcValue};
use {Transport, Error as RpcError};

macro_rules! try_nb {
  ($e:expr) => (match $e {
    Ok(t) => t,
    Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
      return Ok(futures::Async::NotReady)
    }
    Err(e) => {
      warn!("Error: {:?}", e);
      return Err(())
    },
  })
}

/// Error returned while initializing IPC transport
#[derive(Debug)]
pub enum IpcError {
  /// Wrapped IO error
  Io(io::Error),
}

impl From<io::Error> for IpcError {
  fn from(err: io::Error) -> Self { IpcError::Io(err) }
}

type Pending = oneshot::Sender<Result<RpcValue, RpcError>>;

/// Event Loop Handle.
/// NOTE: Event loop is stopped when handle is dropped!
pub struct EventLoopHandle {
  thread: Option<thread::JoinHandle<()>>,
  done: Arc<atomic::AtomicBool>,
}

impl Drop for EventLoopHandle {
  fn drop(&mut self) {
    self.done.store(true, atomic::Ordering::Relaxed);
    self.thread.take()
      .expect("We never touch thread except for drop; drop happens only once; qed")
      .join()
      .expect("Thread should shut down cleanly.");
  }
}

/// Unix Domain Sockets (IPC) transport
pub struct Ipc {
  id: atomic::AtomicUsize,
  write_sender: mpsc::Sender<String>,
  read_sender: mpsc::Sender<(usize, Pending)>,
}

impl Ipc {
  /// Create new IPC transport within existing Event Loop.
  pub fn new<P>(path: P, handle: &reactor::Handle) -> Result<Self, IpcError> where
    P: AsRef<Path>,
  {
    trace!("Connecting to: {:?}", path.as_ref());
    let stream = UnixStream::connect(path, handle)?;
    Self::with_stream(stream, handle)
  }

  /// Creates new IPC transport from existing `UnixStream` and `Handle`
  fn with_stream(stream: UnixStream, handle: &reactor::Handle) -> Result<Self, IpcError> {
    let (read, write) = stream.split();
    let (write_sender, write_receiver) = mpsc::channel(2);
    let (read_sender, read_receiver) = mpsc::channel(2);

    let r = ReadStream {
      read: read,
      incoming: read_receiver,
      pending: Default::default(),
      buffer: vec![],
      current_pos: 0,
    };

    let w = WriteStream {
      write: write,
      incoming: write_receiver,
      state: WriteState::WaitingForRequest,
    };

    handle.spawn(r);
    handle.spawn(w);

    Ok(Ipc {
      id: atomic::AtomicUsize::new(1),
      write_sender: write_sender,
      read_sender: read_sender,
    })
  }

  /// Create new IPC transport with separate event loop.
  /// NOTE: Dropping event loop handle will stop the transport layer!
  pub fn with_event_loop<P>(path: P) -> Result<(EventLoopHandle, Self), IpcError> where
    P: AsRef<Path>,
  {
    let done = Arc::new(atomic::AtomicBool::new(false));
    let (tx, rx) = sync::mpsc::channel();
    let path = path.as_ref().to_owned();
    let done2 = done.clone();

    let eloop = thread::spawn(move || {
      let run = move || {
        let event_loop = reactor::Core::new()?;
        let ipc = Self::new(path, &event_loop.handle())?;
        Ok((ipc, event_loop))
      };

      let send = move |result| {
        tx.send(result).expect("Receiving end is always waiting.");
      };

      let res: Result<_, IpcError> = run();
      match res {
        Err(e) => send(Err(e)),
        Ok((ipc, mut event_loop)) => {
          send(Ok(ipc));

          while !done2.load(atomic::Ordering::Relaxed) {
            event_loop.turn(None);
          }
        },
      }
    });

    rx.recv()
      .expect("Thread is always spawned.")
      .map(|ipc| (
        EventLoopHandle { thread: Some(eloop), done: done },
        ipc,
      ))
  }
}

impl Transport for Ipc {
  type Out = BoxFuture<RpcValue, RpcError>;

  fn execute(&self, method: &str, params: Vec<String>) -> Self::Out {
    let id = self.id.fetch_add(1, atomic::Ordering::Relaxed);
    let request = helpers::build_request(id, method, params);
    debug!("Calling: {}", request);


    // When the response is ready
    let (tx, rx) = futures::oneshot();
    let read_sender = self.read_sender.clone();
    let write_sender = self.write_sender.clone();

    read_sender.send((id, tx))
      .then(|_| write_sender.send(request))
      .then(|_| rx)
      .then(|result| result.unwrap_or(Err(RpcError::Unreachable)))
      .boxed()
  }
}

enum WriteState {
  WaitingForRequest,
  Writing {
    buffer: Vec<u8>,
    current_pos: usize
  },
}

/// Writing part of the IPC transport
/// Awaits new requests using `mpsc::Receiver` and writes them to the socket.
struct WriteStream {
  write: WriteHalf<UnixStream>,
  incoming: mpsc::Receiver<String>,
  state: WriteState
}

impl Future for WriteStream {
  type Item = ();
  type Error = ();

  fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
    let new_state = match self.state {
      WriteState::WaitingForRequest => {
        // Ask for more to write
        let to_send = try_ready!(self.incoming.poll());
        if let Some(to_send) = to_send {
          trace!("Got new message to write: {:?}", to_send);
          WriteState::Writing {
            buffer: to_send.into_bytes(),
            current_pos: 0,
          }
        } else {
          return Ok(futures::Async::NotReady);
        }
      },
      WriteState::Writing { ref buffer, ref mut current_pos } => {
        try_ready!(Ok(self.write.poll_write()));

        // Write everything in the buffer
        while *current_pos < buffer.len() {
          let n = try_nb!(self.write.write(&buffer[*current_pos..]));
          *current_pos += n;
          if n == 0 {
            warn!("Zero write.");
            return Err(()); // zero write?
          }
        }

        WriteState::WaitingForRequest
      },
    };

    self.state = new_state;
    self.poll()
  }
}
/// Reading part of the IPC transport.
/// Reads data on the socket and tries to dispatch it to awaiting requests.
struct ReadStream {
  read: ReadHalf<UnixStream>,
  incoming: mpsc::Receiver<(usize, Pending)>,
  pending: BTreeMap<usize, Pending>,
  buffer: Vec<u8>,
  current_pos: usize,
}

impl Future for ReadStream {
  type Item = ();
  type Error = ();

  fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
    // Check new incoming response channels
    let response = self.incoming.poll();
    if let Ok(futures::Async::Ready(Some((id, response)))) = response {
      self.pending.insert(id, response);
    }

    const DEFAULT_BUF_SIZE: usize = 4096;
    let mut new_write_size = 128;
    loop {
      // Read pending responses
      try_ready!(Ok(self.read.poll_read()));

      if self.current_pos == self.buffer.len() {
        if new_write_size < DEFAULT_BUF_SIZE {
          new_write_size *= 2;
        }
        self.buffer.resize(self.current_pos + new_write_size, 0);
      }

      let read = try_nb!(self.read.read(&mut self.buffer[self.current_pos..]));
      if read == 0 {
        return Ok(futures::Async::NotReady);
      }

      let min = self.current_pos;
      self.current_pos += read;
      if let Some((output, len)) = Self::extract_response(&self.buffer[0..self.current_pos], min) {
        // copy rest of buffer to the beginning
        for i in len..self.current_pos {
          self.buffer.swap(i, i - len);
        }
        self.buffer.truncate(self.current_pos - len + new_write_size);
        self.current_pos = 0;
        self.respond(output);
      }
    }
  }
}

impl ReadStream {
  fn respond(&mut self, output: rpc::Output) {
    let id = match output {
      rpc::Output::Success(ref success) => success.id.clone(),
      rpc::Output::Failure(ref failure) => failure.id.clone(),
    };

    if let rpc::Id::Num(num) = id {
      if let Some(request) = self.pending.remove(&(num as usize)) {
        trace!("Responding to (id: {:?}) with {:?}", num, output);
        request.complete(helpers::to_result_from_output(output));
      } else {
        warn!("Got response for unknown request (id: {:?})", num);
      }
    } else {
      warn!("Got unsupported response (id: {:?})", id);
    }
  }

  fn extract_response(buf: &[u8], min: usize) -> Option<(rpc::Output, usize)> {
    for pos in (min..buf.len()).rev() {
      // Look for end character
      if buf[pos] == b']' || buf[pos] == b'}' {
        // Try to deserialize
        match helpers::to_response_from_slice(&buf[0..pos + 1]) {
          Ok(rpc::Response::Single(output)) => return Some((output, pos)),
          Ok(rpc::Response::Batch(_)) => panic!("Unsupported batch response"),
          // just continue
          _ => {},
        }
      }
    }

    None
  }
}

#[cfg(test)]
mod tests {
  extern crate tokio_core;
  extern crate tokio_uds;

  use std::io::{Read, Write};
  use super::Ipc;
  use futures::{self, Future};
  use rpc;
  use {Transport};

  #[test]
  fn should_send_a_request() {
    // given
    let mut eloop = tokio_core::reactor::Core::new().unwrap();
    let handle = eloop.handle();
    let (server, client) = tokio_uds::UnixStream::pair(&handle).unwrap();
    let ipc = Ipc::with_stream(client, &handle).unwrap();

    eloop.remote().spawn(move |_| {
      struct Task {
        server: tokio_uds::UnixStream,
      }

      impl Future for Task {
        type Item = ();
        type Error = ();
        fn poll(&mut self) -> futures::Poll<(), ()> {
          let mut data = [0; 2048];
          // Read request
          let read = self.server.read(&mut data).unwrap();
          let request = String::from_utf8(data[0..read].to_vec()).unwrap();
          assert_eq!(&request, r#"{"jsonrpc":"2.0","method":"eth_accounts","params":["1"],"id":1}"#);

          // Write response
          let response = r#"{"jsonrpc":"2.0","id":1,"result":"x"}"#;
          self.server.write_all(response.as_bytes()).unwrap();
          self.server.flush().unwrap();

          Ok(futures::Async::Ready(()))
        }
      }

      Task { server: server }
    });

    // when
    let res = ipc.execute("eth_accounts", vec!["1".into()]);

    // then
    assert_eq!(eloop.run(res), Ok(rpc::Value::String("x".into())));
  }
}
