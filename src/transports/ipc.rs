//! IPC Transport for *nix

#[cfg(unix)]
extern crate tokio_uds;

use std::collections::BTreeMap;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::{atomic, Arc};

#[cfg(unix)]
use self::tokio_uds::UnixStream;

use crate::api::SubscriptionId;
use crate::helpers;
use crate::rpc;
use crate::transports::shared::{EventLoopHandle, Response};
use crate::transports::tokio_core::reactor;
use crate::transports::tokio_io::io::{ReadHalf, WriteHalf};
use crate::transports::tokio_io::AsyncRead;
use crate::transports::Result;
use crate::{BatchTransport, DuplexTransport, Error, RequestId, Transport};
use futures::sync::{mpsc, oneshot};
use futures::{self, Future, Stream};
use parking_lot::Mutex;

macro_rules! try_nb {
    ($e:expr) => {
        match $e {
            Ok(t) => t,
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => return Ok(futures::Async::NotReady),
            Err(e) => {
                log::warn!("Unexpected IO error: {:?}", e);
                return Err(());
            }
        }
    };
}

type Pending = oneshot::Sender<Result<Vec<Result<rpc::Value>>>>;

type Subscription = mpsc::UnboundedSender<rpc::Value>;

/// A future representing pending IPC request, resolves to a response.
pub type IpcTask<F> = Response<F, Vec<Result<rpc::Value>>>;

/// Unix Domain Sockets (IPC) transport
#[derive(Debug, Clone)]
pub struct Ipc {
    id: Arc<atomic::AtomicUsize>,
    pending: Arc<Mutex<BTreeMap<RequestId, Pending>>>,
    subscriptions: Arc<Mutex<BTreeMap<SubscriptionId, Subscription>>>,
    write_sender: mpsc::UnboundedSender<Vec<u8>>,
}

impl Ipc {
    /// Create new IPC transport with separate event loop.
    /// NOTE: Dropping event loop handle will stop the transport layer!
    ///
    /// IPC is only available on Unix. On other systems, this always returns an error.
    pub fn new<P>(path: P) -> Result<(EventLoopHandle, Self)>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref().to_owned();
        EventLoopHandle::spawn(move |handle| Self::with_event_loop(&path, &handle).map_err(Into::into))
    }

    /// Create new IPC transport within existing Event Loop.
    ///
    /// IPC is only available on Unix. On other systems, this always returns an error.
    #[cfg(unix)]
    pub fn with_event_loop<P>(path: P, handle: &reactor::Handle) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        log::trace!("Connecting to: {:?}", path.as_ref());
        let stream = UnixStream::connect(path, handle)?;
        Self::with_stream(stream, handle)
    }

    /// Creates new IPC transport from existing `UnixStream` and `Handle`
    #[cfg(unix)]
    fn with_stream(stream: UnixStream, handle: &reactor::Handle) -> Result<Self> {
        let (read, write) = stream.split();
        let (write_sender, write_receiver) = mpsc::unbounded();
        let pending: Arc<Mutex<BTreeMap<RequestId, Pending>>> = Default::default();
        let subscriptions: Arc<Mutex<BTreeMap<SubscriptionId, Subscription>>> = Default::default();

        let r = ReadStream {
            read,
            pending: pending.clone(),
            subscriptions: subscriptions.clone(),
            buffer: vec![],
            current_pos: 0,
        };

        let w = WriteStream {
            write,
            incoming: write_receiver,
            state: WriteState::WaitingForRequest,
        };

        handle.spawn(r);
        handle.spawn(w);

        Ok(Ipc {
            id: Arc::new(atomic::AtomicUsize::new(1)),
            write_sender,
            pending,
            subscriptions,
        })
    }

    #[cfg(not(unix))]
    pub fn with_event_loop<P>(_path: P, _handle: &reactor::Handle) -> Result<Self> {
        return Err(Error::Transport("IPC transport is only supported on Unix".into()).into());
    }

    fn send_request<F, O>(&self, id: RequestId, request: rpc::Request, extract: F) -> IpcTask<F>
    where
        F: Fn(Vec<Result<rpc::Value>>) -> O,
    {
        let request = helpers::to_string(&request);
        log::debug!("[{}] Calling: {}", id, request);
        let (tx, rx) = futures::oneshot();
        self.pending.lock().insert(id, tx);

        let result = self
            .write_sender
            .unbounded_send(request.into_bytes())
            .map_err(|_| Error::Io(io::ErrorKind::BrokenPipe.into()));

        Response::new(id, result, rx, extract)
    }
}

impl Transport for Ipc {
    type Out = IpcTask<fn(Vec<Result<rpc::Value>>) -> Result<rpc::Value>>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        let id = self.id.fetch_add(1, atomic::Ordering::AcqRel);
        let request = helpers::build_request(id, method, params);

        (id, request)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        self.send_request(id, rpc::Request::Single(request), single_response)
    }
}

fn single_response(response: Vec<Result<rpc::Value>>) -> Result<rpc::Value> {
    match response.into_iter().next() {
        Some(res) => res,
        None => Err(Error::InvalidResponse("Expected single, got batch.".into())),
    }
}

impl BatchTransport for Ipc {
    type Batch = IpcTask<fn(Vec<Result<rpc::Value>>) -> Result<Vec<Result<rpc::Value>>>>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)>,
    {
        let mut it = requests.into_iter();
        let (id, first) = it.next().map(|x| (x.0, Some(x.1))).unwrap_or_else(|| (0, None));
        let requests = first.into_iter().chain(it.map(|x| x.1)).collect();
        self.send_request(id, rpc::Request::Batch(requests), Ok)
    }
}

impl DuplexTransport for Ipc {
    type NotificationStream = Box<dyn Stream<Item = rpc::Value, Error = Error> + Send + 'static>;

    fn subscribe(&self, id: &SubscriptionId) -> Self::NotificationStream {
        let (tx, rx) = mpsc::unbounded();
        if self.subscriptions.lock().insert(id.clone(), tx).is_some() {
            log::warn!("Replacing already-registered subscription with id {:?}", id)
        }
        Box::new(rx.map_err(|()| Error::Transport("No data available".into())))
    }

    fn unsubscribe(&self, id: &SubscriptionId) {
        self.subscriptions.lock().remove(id);
    }
}

enum WriteState {
    WaitingForRequest,
    Writing { buffer: Vec<u8>, current_pos: usize },
}

/// Writing part of the IPC transport
/// Awaits new requests using `mpsc::UnboundedReceiver` and writes them to the socket.
#[cfg(unix)]
struct WriteStream {
    write: WriteHalf<UnixStream>,
    incoming: mpsc::UnboundedReceiver<Vec<u8>>,
    state: WriteState,
}

#[cfg(unix)]
impl Future for WriteStream {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        loop {
            self.state = match self.state {
                WriteState::WaitingForRequest => {
                    // Ask for more to write
                    let to_send = try_ready!(self.incoming.poll());
                    if let Some(to_send) = to_send {
                        log::trace!("Got new message to write: {:?}", String::from_utf8_lossy(&to_send));
                        WriteState::Writing {
                            buffer: to_send,
                            current_pos: 0,
                        }
                    } else {
                        return Ok(futures::Async::NotReady);
                    }
                }
                WriteState::Writing {
                    ref buffer,
                    ref mut current_pos,
                } => {
                    // Write everything in the buffer
                    while *current_pos < buffer.len() {
                        let n = try_nb!(self.write.write(&buffer[*current_pos..]));
                        *current_pos += n;
                        if n == 0 {
                            log::warn!("IO Error: Zero write.");
                            return Err(()); // zero write?
                        }
                    }

                    WriteState::WaitingForRequest
                }
            };
        }
    }
}

/// Reading part of the IPC transport.
/// Reads data on the socket and tries to dispatch it to awaiting requests.
#[cfg(unix)]
struct ReadStream {
    read: ReadHalf<UnixStream>,
    pending: Arc<Mutex<BTreeMap<RequestId, Pending>>>,
    subscriptions: Arc<Mutex<BTreeMap<SubscriptionId, Subscription>>>,
    buffer: Vec<u8>,
    current_pos: usize,
}

#[cfg(unix)]
impl Future for ReadStream {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
        const DEFAULT_BUF_SIZE: usize = 4096;
        let mut new_write_size = 128;
        loop {
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

            let mut min = self.current_pos;
            self.current_pos += read;
            while let Some((response, len)) = Self::extract_response(&self.buffer[0..self.current_pos], min) {
                // Respond
                self.respond(response);

                // copy rest of buffer to the beginning
                for i in len..self.current_pos {
                    self.buffer.swap(i, i - len);
                }

                // truncate the buffer
                let new_len = self.current_pos - len;
                self.buffer.truncate(new_len + new_write_size);

                // Set new positions
                self.current_pos = new_len;
                min = 0;
            }
        }
    }
}

enum Message {
    Rpc(Vec<rpc::Output>),
    Notification(rpc::Notification),
}

#[cfg(unix)]
impl ReadStream {
    fn respond(&self, response: Message) {
        match response {
            Message::Rpc(outputs) => {
                let id = match outputs.get(0) {
                    Some(&rpc::Output::Success(ref success)) => success.id.clone(),
                    Some(&rpc::Output::Failure(ref failure)) => failure.id.clone(),
                    None => rpc::Id::Num(0),
                };

                if let rpc::Id::Num(num) = id {
                    if let Some(request) = self.pending.lock().remove(&(num as usize)) {
                        log::trace!("Responding to (id: {:?}) with {:?}", num, outputs);
                        if let Err(err) = request.send(helpers::to_results_from_outputs(outputs)) {
                            log::warn!("Sending a response to deallocated channel: {:?}", err);
                        }
                    } else {
                        log::warn!("Got response for unknown request (id: {:?})", num);
                    }
                } else {
                    log::warn!("Got unsupported response (id: {:?})", id);
                }
            }
            Message::Notification(notification) => {
                if let rpc::Params::Map(params) = notification.params {
                    let id = params.get("subscription");
                    let result = params.get("result");

                    if let (Some(&rpc::Value::String(ref id)), Some(result)) = (id, result) {
                        let id: SubscriptionId = id.clone().into();
                        if let Some(stream) = self.subscriptions.lock().get(&id) {
                            if let Err(e) = stream.unbounded_send(result.clone()) {
                                log::error!("Error sending notification (id: {:?}): {:?}", id, e);
                            }
                        } else {
                            log::warn!("Got notification for unknown subscription (id: {:?})", id);
                        }
                    } else {
                        log::error!("Got unsupported notification (id: {:?})", id);
                    }
                }
            }
        }
    }

    fn extract_response(buf: &[u8], min: usize) -> Option<(Message, usize)> {
        for pos in (min..buf.len()).rev() {
            // Look for end character
            if buf[pos] == b']' || buf[pos] == b'}' {
                // Try to deserialize
                let pos = pos + 1;
                match helpers::to_response_from_slice(&buf[0..pos]) {
                    Ok(rpc::Response::Single(output)) => return Some((Message::Rpc(vec![output]), pos)),
                    Ok(rpc::Response::Batch(outputs)) => return Some((Message::Rpc(outputs), pos)),
                    // just continue
                    _ => {}
                }
                match helpers::to_notification_from_slice(&buf[0..pos]) {
                    Ok(notification) => return Some((Message::Notification(notification), pos)),
                    _ => {}
                }
            }
        }

        None
    }
}

#[cfg(all(test, unix))]
mod tests {
    extern crate tokio_core;
    extern crate tokio_uds;

    use super::Ipc;
    use crate::rpc;
    use crate::Transport;
    use futures::{self, Future};
    use std::io::{self, Read, Write};

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
                    let read = try_nb!(self.server.read(&mut data));
                    let request = String::from_utf8(data[0..read].to_vec()).unwrap();
                    assert_eq!(
                        &request,
                        r#"{"jsonrpc":"2.0","method":"eth_accounts","params":["1"],"id":1}"#
                    );

                    // Write response
                    let response = r#"{"jsonrpc":"2.0","id":1,"result":"x"}"#;
                    self.server.write_all(response.as_bytes()).unwrap();
                    self.server.flush().unwrap();

                    Ok(futures::Async::Ready(()))
                }
            }

            Task { server }
        });

        // when
        let res = ipc.execute("eth_accounts", vec![rpc::Value::String("1".into())]);

        // then
        assert_eq!(eloop.run(res), Ok(rpc::Value::String("x".into())));
    }

    #[test]
    fn should_handle_double_response() {
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
                    let read = try_nb!(self.server.read(&mut data));
                    let request = String::from_utf8(data[0..read].to_vec()).unwrap();
                    assert_eq!(&request, r#"{"jsonrpc":"2.0","method":"eth_accounts","params":["1"],"id":1}{"jsonrpc":"2.0","method":"eth_accounts","params":["1"],"id":2}"#);

                    // Write response
                    let response = r#"{"jsonrpc":"2.0","id":1,"result":"x"}{"jsonrpc":"2.0","id":2,"result":"x"}"#;
                    self.server.write_all(response.as_bytes()).unwrap();
                    self.server.flush().unwrap();

                    Ok(futures::Async::Ready(()))
                }
            }

            Task { server }
        });

        // when
        let res1 = ipc.execute("eth_accounts", vec![rpc::Value::String("1".into())]);
        let res2 = ipc.execute("eth_accounts", vec![rpc::Value::String("1".into())]);

        // then
        assert_eq!(
            eloop.run(res1.join(res2)),
            Ok((rpc::Value::String("x".into()), rpc::Value::String("x".into())))
        );
    }
}
