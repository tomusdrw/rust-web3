//! WebSocket Transport

use self::compat::{TcpStream, TlsStream};
use crate::{api::SubscriptionId, error, helpers, rpc, BatchTransport, DuplexTransport, Error, RequestId, Transport};
use futures::{
    channel::{mpsc, oneshot},
    task::{Context, Poll},
    AsyncRead, AsyncWrite, Future, FutureExt, Stream, StreamExt,
};
use soketto::{
    connection,
    handshake::{Client, ServerResponse},
};
use std::{
    collections::BTreeMap,
    fmt,
    marker::Unpin,
    pin::Pin,
    sync::{atomic, Arc},
};
use url::Url;

impl From<soketto::handshake::Error> for Error {
    fn from(err: soketto::handshake::Error) -> Self {
        Error::Transport(format!("Handshake Error: {:?}", err))
    }
}

impl From<connection::Error> for Error {
    fn from(err: connection::Error) -> Self {
        Error::Transport(format!("Connection Error: {:?}", err))
    }
}

type SingleResult = error::Result<rpc::Value>;
type BatchResult = error::Result<Vec<SingleResult>>;
type Pending = oneshot::Sender<BatchResult>;
type Subscription = mpsc::UnboundedSender<rpc::Value>;

/// Stream, either plain TCP or TLS.
enum MaybeTlsStream<P, T> {
    /// Unencrypted socket stream.
    Plain(P),
    /// Encrypted socket stream.
    #[allow(dead_code)]
    Tls(T),
}

impl<P, T> AsyncRead for MaybeTlsStream<P, T>
where
    P: AsyncRead + AsyncWrite + Unpin,
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_read(self: Pin<&mut Self>, cx: &mut Context, buf: &mut [u8]) -> Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            MaybeTlsStream::Plain(ref mut s) => Pin::new(s).poll_read(cx, buf),
            MaybeTlsStream::Tls(ref mut s) => Pin::new(s).poll_read(cx, buf),
        }
    }
}

impl<P, T> AsyncWrite for MaybeTlsStream<P, T>
where
    P: AsyncRead + AsyncWrite + Unpin,
    T: AsyncRead + AsyncWrite + Unpin,
{
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<Result<usize, std::io::Error>> {
        match self.get_mut() {
            MaybeTlsStream::Plain(ref mut s) => Pin::new(s).poll_write(cx, buf),
            MaybeTlsStream::Tls(ref mut s) => Pin::new(s).poll_write(cx, buf),
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            MaybeTlsStream::Plain(ref mut s) => Pin::new(s).poll_flush(cx),
            MaybeTlsStream::Tls(ref mut s) => Pin::new(s).poll_flush(cx),
        }
    }

    fn poll_close(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<(), std::io::Error>> {
        match self.get_mut() {
            MaybeTlsStream::Plain(ref mut s) => Pin::new(s).poll_close(cx),
            MaybeTlsStream::Tls(ref mut s) => Pin::new(s).poll_close(cx),
        }
    }
}

struct WsServerTask {
    pending: BTreeMap<RequestId, Pending>,
    subscriptions: BTreeMap<SubscriptionId, Subscription>,
    sender: connection::Sender<MaybeTlsStream<TcpStream, TlsStream>>,
    receiver: connection::Receiver<MaybeTlsStream<TcpStream, TlsStream>>,
}

impl WsServerTask {
    /// Create new WebSocket transport.
    pub async fn new(url: &str) -> error::Result<Self> {
        let url = Url::parse(url)?;

        let scheme = match url.scheme() {
            s if s == "ws" || s == "wss" => s,
            s => return Err(error::Error::Transport(format!("Wrong scheme: {}", s))),
        };
        let host = match url.host_str() {
            Some(s) => s,
            None => return Err(error::Error::Transport("Wrong host name".to_string())),
        };
        let port = url.port().unwrap_or(if scheme == "ws" { 80 } else { 443 });
        let addrs = format!("{}:{}", host, port);

        let stream = compat::raw_tcp_stream(addrs).await?;
        stream.set_nodelay(true)?;
        let socket = if scheme == "wss" {
            #[cfg(any(feature = "ws-tls-tokio", feature = "ws-tls-async-std"))]
            {
                let stream = async_native_tls::connect(host, stream).await?;
                MaybeTlsStream::Tls(compat::compat(stream))
            }
            #[cfg(not(any(feature = "ws-tls-tokio", feature = "ws-tls-async-std")))]
            panic!("The library was compiled without TLS support. Enable ws-tls-tokio or ws-tls-async-std feature.");
        } else {
            let stream = compat::compat(stream);
            MaybeTlsStream::Plain(stream)
        };

        let mut client = Client::new(socket, host, url.path());
        let handshake = client.handshake();
        let (sender, receiver) = match handshake.await? {
            ServerResponse::Accepted { .. } => client.into_builder().finish(),
            ServerResponse::Redirect { status_code, location } => {
                return Err(error::Error::Transport(format!(
                    "(code: {}) Unable to follow redirects: {}",
                    status_code, location
                )))
            }
            ServerResponse::Rejected { status_code } => {
                return Err(error::Error::Transport(format!(
                    "(code: {}) Connection rejected.",
                    status_code
                )))
            }
        };

        Ok(Self {
            pending: Default::default(),
            subscriptions: Default::default(),
            sender,
            receiver,
        })
    }

    async fn into_task(self, requests: mpsc::UnboundedReceiver<TransportMessage>) {
        let Self {
            receiver,
            mut sender,
            mut pending,
            mut subscriptions,
        } = self;

        let receiver = as_data_stream(receiver).fuse();
        let requests = requests.fuse();
        pin_mut!(receiver);
        pin_mut!(requests);
        loop {
            select! {
                msg = requests.next() => match msg {
                    Some(TransportMessage::Request { id, request, sender: tx }) => {
                        if pending.insert(id.clone(), tx).is_some() {
                            log::warn!("Replacing a pending request with id {:?}", id);
                        }
                        let res = sender.send_text(request).await;
                        let res2 = sender.flush().await;
                        if let Err(e) = res.and(res2) {
                            // TODO [ToDr] Re-connect.
                            log::error!("WS connection error: {:?}", e);
                            pending.remove(&id);
                        }
                    }
                    Some(TransportMessage::Subscribe { id, sink }) => {
                        if subscriptions.insert(id.clone(), sink).is_some() {
                            log::warn!("Replacing already-registered subscription with id {:?}", id);
                        }
                    }
                    Some(TransportMessage::Unsubscribe { id }) => {
                        if subscriptions.remove(&id).is_none() {
                            log::warn!("Unsubscribing from non-existent subscription with id {:?}", id);
                        }
                    }
                    None => {}
                },
                res = receiver.next() => match res {
                    Some(Ok(data)) => {
                        handle_message(&data, &subscriptions, &mut pending);
                    },
                    Some(Err(e)) => {
                        log::error!("WS connection error: {:?}", e);
                        break;
                    },
                    None => break,
                },
                complete => break,
            }
        }
    }
}

fn as_data_stream<T: Unpin + futures::AsyncRead + futures::AsyncWrite>(
    receiver: soketto::connection::Receiver<T>,
) -> impl Stream<Item = Result<Vec<u8>, soketto::connection::Error>> {
    futures::stream::unfold(receiver, |mut receiver| async move {
        let mut data = Vec::new();
        Some(match receiver.receive_data(&mut data).await {
            Ok(_) => (Ok(data), receiver),
            Err(e) => (Err(e), receiver),
        })
    })
}

fn handle_message(
    data: &[u8],
    subscriptions: &BTreeMap<SubscriptionId, Subscription>,
    pending: &mut BTreeMap<RequestId, Pending>,
) {
    log::trace!("Message received: {:?}", data);
    if let Ok(notification) = helpers::to_notification_from_slice(data) {
        if let rpc::Params::Map(params) = notification.params {
            let id = params.get("subscription");
            let result = params.get("result");

            if let (Some(&rpc::Value::String(ref id)), Some(result)) = (id, result) {
                let id: SubscriptionId = id.clone().into();
                if let Some(stream) = subscriptions.get(&id) {
                    if let Err(e) = stream.unbounded_send(result.clone()) {
                        log::error!("Error sending notification: {:?} (id: {:?}", e, id);
                    }
                } else {
                    log::warn!("Got notification for unknown subscription (id: {:?})", id);
                }
            } else {
                log::error!("Got unsupported notification (id: {:?})", id);
            }
        }
    } else {
        let response = helpers::to_response_from_slice(data);
        let outputs = match response {
            Ok(rpc::Response::Single(output)) => vec![output],
            Ok(rpc::Response::Batch(outputs)) => outputs,
            _ => vec![],
        };

        let id = match outputs.get(0) {
            Some(&rpc::Output::Success(ref success)) => success.id.clone(),
            Some(&rpc::Output::Failure(ref failure)) => failure.id.clone(),
            None => rpc::Id::Num(0),
        };

        if let rpc::Id::Num(num) = id {
            if let Some(request) = pending.remove(&(num as usize)) {
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
}

enum TransportMessage {
    Request {
        id: RequestId,
        request: String,
        sender: oneshot::Sender<BatchResult>,
    },
    Subscribe {
        id: SubscriptionId,
        sink: mpsc::UnboundedSender<rpc::Value>,
    },
    Unsubscribe {
        id: SubscriptionId,
    },
}

/// WebSocket transport
#[derive(Clone)]
pub struct WebSocket {
    id: Arc<atomic::AtomicUsize>,
    requests: mpsc::UnboundedSender<TransportMessage>,
}

impl fmt::Debug for WebSocket {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("WebSocket").field("id", &self.id).finish()
    }
}

impl WebSocket {
    /// Create new WebSocket transport.
    pub async fn new(url: &str) -> error::Result<Self> {
        let id = Arc::new(atomic::AtomicUsize::new(1));
        let task = WsServerTask::new(url).await?;
        // TODO [ToDr] Not unbounded?
        let (sink, stream) = mpsc::unbounded();
        // Spawn background task for the transport.
        #[cfg(feature = "ws-tokio")]
        tokio::spawn(task.into_task(stream));
        #[cfg(feature = "ws-async-std")]
        async_std::task::spawn(task.into_task(stream));

        Ok(Self { id, requests: sink })
    }

    fn send(&self, msg: TransportMessage) -> error::Result {
        self.requests.unbounded_send(msg).map_err(dropped_err)
    }

    fn send_request(&self, id: RequestId, request: rpc::Request) -> error::Result<oneshot::Receiver<BatchResult>> {
        let request = helpers::to_string(&request);
        log::debug!("[{}] Calling: {}", id, request);
        let (sender, receiver) = oneshot::channel();
        self.send(TransportMessage::Request { id, request, sender })?;
        Ok(receiver)
    }
}

fn dropped_err<T>(_: T) -> error::Error {
    Error::Transport("Cannot send request. Internal task finished.".into())
}

fn batch_to_single(response: BatchResult) -> SingleResult {
    match response?.into_iter().next() {
        Some(res) => res,
        None => Err(Error::InvalidResponse("Expected single, got batch.".into())),
    }
}

fn batch_to_batch(res: BatchResult) -> BatchResult {
    res
}

enum ResponseState {
    Receiver(Option<error::Result<oneshot::Receiver<BatchResult>>>),
    Waiting(oneshot::Receiver<BatchResult>),
}

/// A WS resonse wrapper.
pub struct Response<R, T> {
    extract: T,
    state: ResponseState,
    _data: std::marker::PhantomData<R>,
}

impl<R, T> Response<R, T> {
    fn new(response: error::Result<oneshot::Receiver<BatchResult>>, extract: T) -> Self {
        Self {
            extract,
            state: ResponseState::Receiver(Some(response)),
            _data: Default::default(),
        }
    }
}

impl<R, T> Future for Response<R, T>
where
    R: Unpin + 'static,
    T: Fn(BatchResult) -> error::Result<R> + Unpin + 'static,
{
    type Output = error::Result<R>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        loop {
            match self.state {
                ResponseState::Receiver(ref mut res) => {
                    let receiver = res.take().expect("Receiver state is active only once; qed")?;
                    self.state = ResponseState::Waiting(receiver)
                }
                ResponseState::Waiting(ref mut future) => {
                    let response = ready!(future.poll_unpin(cx)).map_err(dropped_err)?;
                    return Poll::Ready((self.extract)(response));
                }
            }
        }
    }
}

impl Transport for WebSocket {
    type Out = Response<rpc::Value, fn(BatchResult) -> SingleResult>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        let id = self.id.fetch_add(1, atomic::Ordering::AcqRel);
        let request = helpers::build_request(id, method, params);

        (id, request)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        let response = self.send_request(id, rpc::Request::Single(request));
        Response::new(response, batch_to_single)
    }
}

impl BatchTransport for WebSocket {
    type Batch = Response<Vec<SingleResult>, fn(BatchResult) -> BatchResult>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)>,
    {
        let mut it = requests.into_iter();
        let (id, first) = it.next().map(|x| (x.0, Some(x.1))).unwrap_or_else(|| (0, None));
        let requests = first.into_iter().chain(it.map(|x| x.1)).collect();
        let response = self.send_request(id, rpc::Request::Batch(requests));
        Response::new(response, batch_to_batch)
    }
}

impl DuplexTransport for WebSocket {
    type NotificationStream = mpsc::UnboundedReceiver<rpc::Value>;

    fn subscribe(&self, id: SubscriptionId) -> error::Result<Self::NotificationStream> {
        // TODO [ToDr] Not unbounded?
        let (sink, stream) = mpsc::unbounded();
        self.send(TransportMessage::Subscribe { id, sink })?;
        Ok(stream)
    }

    fn unsubscribe(&self, id: SubscriptionId) -> error::Result {
        self.send(TransportMessage::Unsubscribe { id })
    }
}

/// Compatibility layer between async-std and tokio
#[cfg(feature = "ws-async-std")]
#[doc(hidden)]
pub mod compat {
    pub use async_std::net::{TcpListener, TcpStream};
    /// TLS stream type for async-std runtime.
    #[cfg(feature = "ws-tls-async-std")]
    pub type TlsStream = async_native_tls::TlsStream<TcpStream>;
    /// Dummy TLS stream type.
    #[cfg(not(feature = "ws-tls-async-std"))]
    pub type TlsStream = TcpStream;

    /// Create new TcpStream object.
    pub async fn raw_tcp_stream(addrs: String) -> std::io::Result<TcpStream> {
        TcpStream::connect(addrs).await
    }

    /// Wrap given argument into compatibility layer.
    #[inline(always)]
    pub fn compat<T>(t: T) -> T {
        t
    }
}

/// Compatibility layer between async-std and tokio
#[cfg(feature = "ws-tokio")]
pub mod compat {
    /// async-std compatible TcpStream.
    pub type TcpStream = Compat<tokio::net::TcpStream>;
    /// async-std compatible TcpListener.
    pub type TcpListener = tokio::net::TcpListener;
    /// TLS stream type for tokio runtime.
    #[cfg(feature = "ws-tls-tokio")]
    pub type TlsStream = Compat<async_native_tls::TlsStream<tokio::net::TcpStream>>;
    /// Dummy TLS stream type.
    #[cfg(not(feature = "ws-tls-tokio"))]
    pub type TlsStream = TcpStream;

    use std::{
        io,
        pin::Pin,
        task::{Context, Poll},
    };

    /// Create new TcpStream object.
    pub async fn raw_tcp_stream(addrs: String) -> io::Result<tokio::net::TcpStream> {
        Ok(tokio::net::TcpStream::connect(addrs).await?)
    }

    /// Wrap given argument into compatibility layer.
    pub fn compat<T>(t: T) -> Compat<T> {
        Compat(t)
    }

    /// Compatibility layer.
    pub struct Compat<T>(T);
    impl<T: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for Compat<T> {
        fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<Result<usize, io::Error>> {
            tokio::io::AsyncWrite::poll_write(Pin::new(&mut self.0), cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
            tokio::io::AsyncWrite::poll_flush(Pin::new(&mut self.0), cx)
        }

        fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
            tokio::io::AsyncWrite::poll_shutdown(Pin::new(&mut self.0), cx)
        }
    }

    impl<T: tokio::io::AsyncWrite + Unpin> futures::AsyncWrite for Compat<T> {
        fn poll_write(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &[u8]) -> Poll<io::Result<usize>> {
            tokio::io::AsyncWrite::poll_write(Pin::new(&mut self.0), cx, buf)
        }

        fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            tokio::io::AsyncWrite::poll_flush(Pin::new(&mut self.0), cx)
        }

        fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            tokio::io::AsyncWrite::poll_shutdown(Pin::new(&mut self.0), cx)
        }
    }

    impl<T: tokio::io::AsyncRead + Unpin> futures::AsyncRead for Compat<T> {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
            tokio::io::AsyncRead::poll_read(Pin::new(&mut self.0), cx, buf)
        }
    }

    impl<T: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for Compat<T> {
        fn poll_read(mut self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut [u8]) -> Poll<io::Result<usize>> {
            tokio::io::AsyncRead::poll_read(Pin::new(&mut self.0), cx, buf)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{rpc, Transport};
    use futures::{
        io::{BufReader, BufWriter},
        StreamExt,
    };
    use soketto::handshake;

    #[test]
    fn bounds_matching() {
        fn async_rw<T: AsyncRead + AsyncWrite>() {}

        async_rw::<TcpStream>();
        async_rw::<MaybeTlsStream<TcpStream, TlsStream>>();
    }

    #[tokio::test]
    async fn should_send_a_request() {
        let _ = env_logger::try_init();
        // given
        let addr = "127.0.0.1:3000";
        let listener = futures::executor::block_on(compat::TcpListener::bind(addr)).expect("Failed to bind");
        println!("Starting the server.");
        tokio::spawn(server(listener, addr));

        let endpoint = "ws://127.0.0.1:3000";
        let ws = WebSocket::new(endpoint).await.unwrap();

        // when
        let res = ws.execute("eth_accounts", vec![rpc::Value::String("1".into())]);

        // then
        assert_eq!(res.await, Ok(rpc::Value::String("x".into())));
    }

    async fn server(mut listener: compat::TcpListener, addr: &str) {
        let mut incoming = listener.incoming();
        println!("Listening on: {}", addr);
        while let Some(Ok(socket)) = incoming.next().await {
            let socket = compat::compat(socket);
            let mut server = handshake::Server::new(BufReader::new(BufWriter::new(socket)));
            let key = {
                let req = server.receive_request().await.unwrap();
                req.into_key()
            };
            let accept = handshake::server::Response::Accept {
                key: &key,
                protocol: None,
            };
            server.send_response(&accept).await.unwrap();
            let (mut sender, mut receiver) = server.into_builder().finish();
            loop {
                let mut data = Vec::new();
                match receiver.receive_data(&mut data).await {
                    Ok(data_type) if data_type.is_text() => {
                        assert_eq!(
                            std::str::from_utf8(&data),
                            Ok(r#"{"jsonrpc":"2.0","method":"eth_accounts","params":["1"],"id":1}"#)
                        );
                        sender
                            .send_text(r#"{"jsonrpc":"2.0","id":1,"result":"x"}"#)
                            .await
                            .unwrap();
                        sender.flush().await.unwrap();
                    }
                    Err(soketto::connection::Error::Closed) => break,
                    e => panic!("Unexpected data: {:?}", e),
                }
            }
        }
    }
}
