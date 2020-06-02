//! WebSocket Transport

use std::collections::BTreeMap;
use std::sync::{atomic, Arc};

use crate::api::SubscriptionId;
use crate::error;
use crate::helpers;
use crate::rpc;
use crate::{BatchTransport, DuplexTransport, Error, RequestId, Transport};
use futures::channel::{mpsc, oneshot};
use futures::{future, StreamExt, Stream, Future, FutureExt};
use parking_lot::Mutex;

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

type BatchResult = error::Result<Vec<error::Result<rpc::Value>>>;
type Pending = oneshot::Sender<BatchResult>;
type Subscription = mpsc::UnboundedSender<rpc::Value>;

/// WebSocket transport
#[derive(Debug, Clone)]
pub struct WebSocket {
    id: Arc<atomic::AtomicUsize>,
    pending: Arc<Mutex<BTreeMap<RequestId, Pending>>>,
    subscriptions: Arc<Mutex<BTreeMap<SubscriptionId, Subscription>>>,
    sender: connection::Sender<TcpStream>,
}

impl WebSocket {
    /// Create new WebSocket transport.
    pub async fn new(url: &str) -> error::Result<Self> {
        let socket = TcpStream::connect(url).await?;
        let mut client = Client::new(socket, "localhost", "web3");
        let (sender, receiver) = match client.handshake().await? {
            ServerResponse::Accepted { .. } => client.into_builder().finish(),
            ServerResponse::Redirect { status_code, location } => return Err(error::Error::Transport(format!(
                    "(code: {}) Unable to follow redirects: {}", status_code, location
            ))),
            ServerResponse::Rejected { status_code } => return Err(error::Error::Transport(format!(
                    "(code: {}) Connection rejected.", status_code
            ))),
        };

        let pending: Arc<Mutex<BTreeMap<RequestId, Pending>>> = Default::default();
        let subscriptions: Arc<Mutex<BTreeMap<SubscriptionId, Subscription>>> = Default::default();

        let pending_ = pending.clone();
        let subscriptions_ = subscriptions.clone();
        let recv_future = connection::into_stream(receiver).for_each(move |message| {
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
                                if let Some(stream) = subscriptions_.lock().get(&id) {
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
                        if let Some(request) = pending_.lock().remove(&(num as usize)) {
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

        async_std::task::spawn(recv_future);

        Ok(Self {
            id: Arc::new(atomic::AtomicUsize::new(1)),
            pending,
            subscriptions,
            sender,
        })
    }

    async fn send_request(&mut self, id: RequestId, request: rpc::Request) -> BatchResult {
        let request = helpers::to_string(&request);
        log::debug!("[{}] Calling: {}", id, request);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().insert(id, tx);

        let result = self
            .sender
            .send_text(request)
            .await?;

        rx.await.map_err(|_| Error::Transport("Dropped.".into()))?
    }
}

impl Transport for WebSocket {
    type Out = Box<dyn Future<Output = error::Result<rpc::Value>> + Send + Unpin>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        let id = self.id.fetch_add(1, atomic::Ordering::AcqRel);
        let request = helpers::build_request(id, method, params);

        (id, request)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        let response = self.send_request(id, rpc::Request::Single(request));
        Box::new(response.map(|response| {
            match response.into_iter().next() {
                Some(res) => Ok(res),
                None => Err(Error::InvalidResponse("Expected single, got batch.".into())),
            }
        }))
    }
}

impl BatchTransport for WebSocket {
    type Batch = Box<dyn Future<Output = BatchResult> + Send + Unpin>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)>,
    {
        let mut it = requests.into_iter();
        let (id, first) = it.next().map(|x| (x.0, Some(x.1))).unwrap_or_else(|| (0, None));
        let requests = first.into_iter().chain(it.map(|x| x.1)).collect();
        let response = self.send_request(id, rpc::Request::Batch(requests));
        Box::new(response)
    }
}

impl DuplexTransport for WebSocket {
    type NotificationStream = Box<dyn Stream<Item = error::Result<rpc::Value>> + Send + Unpin>;

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

#[cfg(test)]
mod tests {
    extern crate websocket;

    use self::websocket::message::OwnedMessage;
    use self::websocket::r#async::Server;
    use self::websocket::server::InvalidConnection;
    use super::WebSocket;
    use crate::rpc;
    use crate::Transport;
    use futures::{Future, Sink, Stream};

    #[test]
    fn should_send_a_request() {
        // given
        let mut eloop = tokio_core::reactor::Core::new().unwrap();
        let handle = eloop.handle();
        let server = Server::bind("localhost:3000", &handle).unwrap();
        let f = {
            let handle_ = handle.clone();
            server
                .incoming()
                .take(1)
                .map_err(|InvalidConnection { error, .. }| error)
                .for_each(move |(upgrade, addr)| {
                    log::trace!("Got a connection from {}", addr);
                    let f = upgrade.accept().and_then(|(s, _)| {
                        let (sink, stream) = s.split();

                        stream
                            .take_while(|m| Ok(!m.is_close()))
                            .filter_map(|m| match m {
                                OwnedMessage::Ping(p) => Some(OwnedMessage::Pong(p)),
                                OwnedMessage::Pong(_) => None,
                                OwnedMessage::Text(t) => {
                                    assert_eq!(t, r#"{"jsonrpc":"2.0","method":"eth_accounts","params":["1"],"id":1}"#);
                                    Some(OwnedMessage::Text(
                                        r#"{"jsonrpc":"2.0","id":1,"result":"x"}"#.to_owned(),
                                    ))
                                }
                                _ => None,
                            })
                            .forward(sink)
                            .and_then(|(_, sink)| sink.send(OwnedMessage::Close(None)))
                    });

                    handle_.spawn(f.map(|_| ()).map_err(|_| ()));

                    Ok(())
                })
        };
        handle.spawn(f.map_err(|_| ()));

        let ws = WebSocket::with_event_loop("ws://localhost:3000", &handle).unwrap();

        // when
        let res = ws.execute("eth_accounts", vec![rpc::Value::String("1".into())]);

        // then
        assert_eq!(eloop.run(res), Ok(rpc::Value::String("x".into())));
    }
}
