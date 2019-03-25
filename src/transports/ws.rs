//! WebSocket Transport

extern crate websocket;

use std::collections::BTreeMap;
use std::sync::{atomic, Arc};

use self::websocket::url::Url;
use self::websocket::{ClientBuilder, OwnedMessage};
use crate::api::SubscriptionId;
use crate::helpers;
use crate::rpc;
use crate::transports::shared::{EventLoopHandle, Response};
use crate::transports::tokio_core::reactor;
use crate::transports::Result;
use crate::{BatchTransport, DuplexTransport, Error, RequestId, Transport};
use futures::sync::{mpsc, oneshot};
use futures::{self, Future, Sink, Stream};
use parking_lot::Mutex;

impl From<websocket::WebSocketError> for Error {
    fn from(err: websocket::WebSocketError) -> Self {
        Error::Transport(format!("{:?}", err)).into()
    }
}

impl From<websocket::client::ParseError> for Error {
    fn from(err: websocket::client::ParseError) -> Self {
        Error::Transport(format!("{:?}", err)).into()
    }
}

type Pending = oneshot::Sender<Result<Vec<Result<rpc::Value>>>>;

type Subscription = mpsc::UnboundedSender<rpc::Value>;

/// A future representing pending WebSocket request, resolves to a response.
pub type WsTask<F> = Response<F, Vec<Result<rpc::Value>>>;

/// WebSocket transport
#[derive(Debug, Clone)]
pub struct WebSocket {
    id: Arc<atomic::AtomicUsize>,
    url: Url,
    pending: Arc<Mutex<BTreeMap<RequestId, Pending>>>,
    subscriptions: Arc<Mutex<BTreeMap<SubscriptionId, Subscription>>>,
    write_sender: mpsc::UnboundedSender<OwnedMessage>,
}

impl WebSocket {
    /// Create new WebSocket transport with separate event loop.
    /// NOTE: Dropping event loop handle will stop the transport layer!
    pub fn new(url: &str) -> Result<(EventLoopHandle, Self)> {
        let url = url.to_owned();
        EventLoopHandle::spawn(move |handle| Self::with_event_loop(&url, &handle).map_err(Into::into))
    }

    /// Create new WebSocket transport within existing Event Loop.
    pub fn with_event_loop(url: &str, handle: &reactor::Handle) -> Result<Self> {
        trace!("Connecting to: {:?}", url);

        let url: Url = url.parse()?;
        let pending: Arc<Mutex<BTreeMap<RequestId, Pending>>> = Default::default();
        let subscriptions: Arc<Mutex<BTreeMap<SubscriptionId, Subscription>>> = Default::default();
        let (write_sender, write_receiver) = mpsc::unbounded();

        let ws_future = {
            let pending_ = pending.clone();
            let subscriptions_ = subscriptions.clone();
            let write_sender_ = write_sender.clone();

            ClientBuilder::from_url(&url).async_connect(None, handle).from_err::<Error>().map(|(duplex, _)| duplex.split()).and_then(move |(sink, stream)| {
                let reader = stream.from_err::<Error>().for_each(move |message| {
                    trace!("Message received: {:?}", message);

                    match message {
                        OwnedMessage::Close(e) => write_sender_.unbounded_send(OwnedMessage::Close(e)).map_err(|_| Error::Transport("Error sending close message".into()).into()),
                        OwnedMessage::Ping(d) => write_sender_.unbounded_send(OwnedMessage::Pong(d)).map_err(|_| Error::Transport("Error sending pong message".into()).into()),
                        OwnedMessage::Text(t) => {
                            if let Ok(notification) = helpers::to_notification_from_slice(t.as_bytes()) {
                                if let Some(rpc::Params::Map(params)) = notification.params {
                                    let id = params.get("subscription");
                                    let result = params.get("result");

                                    if let (Some(&rpc::Value::String(ref id)), Some(result)) = (id, result) {
                                        let id: SubscriptionId = id.clone().into();
                                        if let Some(stream) = subscriptions_.lock().get(&id) {
                                            return stream.unbounded_send(result.clone()).map_err(|_| Error::Transport("Error sending notification".into()).into());
                                        } else {
                                            warn!("Got notification for unknown subscription (id: {:?})", id);
                                        }
                                    } else {
                                        error!("Got unsupported notification (id: {:?})", id);
                                    }
                                }

                                return Ok(());
                            }

                            let response = helpers::to_response_from_slice(t.as_bytes());
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
                                    trace!("Responding to (id: {:?}) with {:?}", num, outputs);
                                    if let Err(err) = request.send(helpers::to_results_from_outputs(outputs)) {
                                        warn!("Sending a response to deallocated channel: {:?}", err);
                                    }
                                } else {
                                    warn!("Got response for unknown request (id: {:?})", num);
                                }
                            } else {
                                warn!("Got unsupported response (id: {:?})", id);
                            }

                            Ok(())
                        }
                        _ => Ok(()),
                    }
                });

                let writer = sink.sink_from_err().send_all(write_receiver.map_err(|_| websocket::WebSocketError::NoDataAvailable)).map(|_| ());

                reader.join(writer)
            })
        };

        handle.spawn(ws_future.map(|_| ()).map_err(|err| {
            error!("WebSocketError: {:?}", err);
        }));

        Ok(Self { id: Arc::new(atomic::AtomicUsize::new(1)), url: url, pending, subscriptions, write_sender })
    }

    fn send_request<F, O>(&self, id: RequestId, request: rpc::Request, extract: F) -> WsTask<F>
    where
        F: Fn(Vec<Result<rpc::Value>>) -> O,
    {
        let request = helpers::to_string(&request);
        debug!("[{}] Calling: {}", id, request);
        let (tx, rx) = futures::oneshot();
        self.pending.lock().insert(id, tx);

        let result = self.write_sender.unbounded_send(OwnedMessage::Text(request)).map_err(|_| Error::Transport("Error sending request".into()).into());

        Response::new(id, result, rx, extract)
    }
}

impl Transport for WebSocket {
    type Out = WsTask<fn(Vec<Result<rpc::Value>>) -> Result<rpc::Value>>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        let id = self.id.fetch_add(1, atomic::Ordering::AcqRel);
        let request = helpers::build_request(id, method, params);

        (id, request)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        self.send_request(id, rpc::Request::Single(request), |response| match response.into_iter().next() {
            Some(res) => res,
            None => Err(Error::InvalidResponse("Expected single, got batch.".into()).into()),
        })
    }
}

impl BatchTransport for WebSocket {
    type Batch = WsTask<fn(Vec<Result<rpc::Value>>) -> Result<Vec<Result<rpc::Value>>>>;

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

impl DuplexTransport for WebSocket {
    type NotificationStream = Box<Stream<Item = rpc::Value, Error = Error> + Send + 'static>;

    fn subscribe(&self, id: &SubscriptionId) -> Self::NotificationStream {
        let (tx, rx) = mpsc::unbounded();
        if self.subscriptions.lock().insert(id.clone(), tx).is_some() {
            warn!("Replacing already-registered subscription with id {:?}", id)
        }
        Box::new(rx.map_err(|()| Error::Transport("No data available".into()).into()))
    }

    fn unsubscribe(&self, id: &SubscriptionId) {
        self.subscriptions.lock().remove(id);
    }
}

#[cfg(test)]
mod tests {
    extern crate tokio_core;
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
            server.incoming().take(1).map_err(|InvalidConnection { error, .. }| error).for_each(move |(upgrade, addr)| {
                trace!("Got a connection from {}", addr);
                let f = upgrade.accept().and_then(|(s, _)| {
                    let (sink, stream) = s.split();

                    stream
                        .take_while(|m| Ok(!m.is_close()))
                        .filter_map(|m| match m {
                            OwnedMessage::Ping(p) => Some(OwnedMessage::Pong(p)),
                            OwnedMessage::Pong(_) => None,
                            OwnedMessage::Text(t) => {
                                assert_eq!(t, r#"{"jsonrpc":"2.0","method":"eth_accounts","params":["1"],"id":1}"#);
                                Some(OwnedMessage::Text(r#"{"jsonrpc":"2.0","id":1,"result":"x"}"#.to_owned()))
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
