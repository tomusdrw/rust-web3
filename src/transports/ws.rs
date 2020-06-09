//! WebSocket Transport

use std::collections::BTreeMap;
use std::sync::{atomic, Arc};
use std::{pin::Pin, fmt};

use crate::api::SubscriptionId;
use crate::error;
use crate::helpers;
use crate::rpc;
use crate::{BatchTransport, DuplexTransport, Error, RequestId, Transport};
use futures::channel::{mpsc, oneshot};
use futures::{future, StreamExt, Future, FutureExt, task::{Poll, Context}};

use async_std::net::TcpStream;
use soketto::connection;
use soketto::data::Incoming;
use soketto::handshake::{Client, ServerResponse};

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

struct WsServerTask {
    pending: BTreeMap<RequestId, Pending>,
    subscriptions: BTreeMap<SubscriptionId, Subscription>,
    sender: connection::Sender<TcpStream>,
    receiver: connection::Receiver<TcpStream>,
}

impl WsServerTask {
    /// Create new WebSocket transport.
    pub async fn new(url: &str) -> error::Result<Self> {
        let url = url.trim_start_matches("ws://");

        let socket = TcpStream::connect(url).await?;
        let mut client = Client::new(socket, url, "/");
        let handshake = client.handshake();
        println!("Awaiting handshake");
        let (sender, receiver) = match handshake.await? {
            ServerResponse::Accepted { .. } => client.into_builder().finish(),
            ServerResponse::Redirect { status_code, location } => return Err(error::Error::Transport(format!(
                    "(code: {}) Unable to follow redirects: {}", status_code, location
            ))),
            ServerResponse::Rejected { status_code } => return Err(error::Error::Transport(format!(
                    "(code: {}) Connection rejected.", status_code
            ))),
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

        let recv_future = connection::into_stream(receiver).for_each(|message| {
            let message = match message {
                Ok(m) => m,
                Err(e) => {
                    log::error!("WebSocket Error: {:?}", e);
                    return future::ready(());
                },
            };
            log::trace!("Message received: {:?}", message);
            match message {
                Incoming::Pong(_) => {},
                Incoming::Data(t) => {
                    if let Ok(notification) = helpers::to_notification_from_slice(t.as_ref()) {
                        if let rpc::Params::Map(params) = notification.params {
                            let id = params.get("subscription");
                            let result = params.get("result");

                            if let (Some(&rpc::Value::String(ref id)), Some(result)) = (id, result) {
                                let id: SubscriptionId = id.clone().into();
                                if let Some(stream) = subscriptions.get(&id) {
                                    return future::ready(stream.unbounded_send(result.clone()).unwrap_or_else(|e| {
                                        log::error!("Error sending notification: {:?} (id: {:?}", e, id);
                                    }));
                                } else {
                                    log::warn!("Got notification for unknown subscription (id: {:?})", id);
                                }
                            } else {
                                log::error!("Got unsupported notification (id: {:?})", id);
                            }
                        }

                        return future::ready(());
                    }

                    let response = helpers::to_response_from_slice(t.as_ref());
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

            future::ready(())
        });

        let recv_future = recv_future.fuse();
        let requests = requests.fuse();
        pin_mut!(recv_future);
        pin_mut!(requests);
        select! {
            msg = requests.next() => match msg {
                Some(TransportMessage::Request { id, request, sender: tx }) => {
                    if pending.insert(id.clone(), tx).is_some() {
                        log::warn!("Replacing a pending request with id {:?}", id);
                    }
                    if let Err(e) = sender.send_text(request).await {
                        // TODO [ToDr] Re-connect.
                        log::error!("WS connection error: {:?}", e);
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
            _ = recv_future => {},
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
        fmt.debug_struct("WebSocket")
            .field("id", &self.id)
            .finish()
    }
}

impl WebSocket {
    /// Create new WebSocket transport.
    pub async fn new(url: &str) -> error::Result<Self> {
        let id = Arc::new(atomic::AtomicUsize::default());
        let task = WsServerTask::new(url).await?;
        // TODO [ToDr] Not unbounded?
        let (sink, stream) = mpsc::unbounded();
        // Spawn background task for the transport.
        async_std::task::spawn(task.into_task(
            stream,
        ));

        Ok(Self {
            id,
            requests: sink,
        })
    }

    fn send(&self, msg: TransportMessage) -> error::Result {
        self.requests.unbounded_send(msg)
            .map_err(dropped_err)
    }

    fn send_request(&self, id: RequestId, request: rpc::Request) -> error::Result<oneshot::Receiver<BatchResult>> {
        let request = helpers::to_string(&request);
        log::debug!("[{}] Calling: {}", id, request);
        let (sender, receiver) = oneshot::channel();
        self.send(TransportMessage::Request {
            id,
            request,
            sender,
        })?;
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

fn batch_to_batch(res: BatchResult) -> BatchResult { res }

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

impl<R, T> Future for Response<R, T> where
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
                    let response = ready!(Pin::new(future).poll(cx))
                        .map_err(dropped_err)?;
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
        self.send(TransportMessage::Subscribe {
            id,
            sink,
        })?;
        Ok(stream)
    }

    fn unsubscribe(&self, id: SubscriptionId) -> error::Result {
        self.send(TransportMessage::Unsubscribe {
            id,
        })
    }
}

#[cfg(test)]
mod tests {
    use async_std::net::TcpListener;
    use crate::{rpc, Transport};
    use futures::StreamExt;
    use super::WebSocket;
    use futures::io::{BufReader, BufWriter};
    use soketto::handshake;

    #[tokio::test(core_threads=2)]
    #[ignore]
    async fn should_send_a_request() {
        let _ = env_logger::try_init();
        // given
        println!("Starting a server");
        tokio::spawn(server("127.0.0.1:3000"));

        println!("Connecting");
        let ws = WebSocket::new("127.0.0.1:3000");
        println!("Awaiting connection");
        let ws = ws.await.unwrap();

        // when
        println!("Making a request");
        let res = ws.execute("eth_accounts", vec![rpc::Value::String("1".into())]);

        // then
        println!("Awaiting response");
        assert_eq!(res.await, Ok(rpc::Value::String("x".into())));
    }

    async fn server(addr: &str) {
        let listener = TcpListener::bind(addr).await.expect("Failed to bind");
        let mut incoming = listener.incoming();
        println!("Listening on: {}", addr);
        while let Some(Ok(socket)) = incoming.next().await {
            println!("Accepting connection");
            let mut server = handshake::Server::new(
                BufReader::new(BufWriter::new(socket))
            );
            println!("Retrieving handshake");
            let key = {
                let req = server.receive_request().await.unwrap();
                req.into_key()
            };
            println!("Responding to handshake");
            let accept = handshake::server::Response::Accept { key: &key, protocol: None };
            server.send_response(&accept).await.unwrap();
            println!("Listening for data.");
            let (mut sender, mut receiver) = server.into_builder().finish();
            loop {
                match receiver.receive_data().await {
                    Ok(data) if data.is_text() => {
                        assert_eq!(
                            data.as_ref(),
                            r#"{"jsonrpc":"2.0","method":"eth_accounts","params":["1"],"id":1}"#.as_bytes()
                        );
                        sender.send_text(r#"{"jsonrpc":"2.0","id":1,"result":"x"}"#).await.unwrap();
                        sender.flush().await.unwrap();
                    },
                    Err(soketto::connection::Error::Closed) => break,
                    e => panic!("Unexpected data: {:?}", e),
                }
            }
        }
    }
}
