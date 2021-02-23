//! IPC transport

use crate::{api::SubscriptionId, helpers, BatchTransport, DuplexTransport, Error, RequestId, Result, Transport};
use futures::future::{join_all, JoinAll};
use jsonrpc_core as rpc;
use std::{
    collections::BTreeMap,
    path::Path,
    pin::Pin,
    sync::{atomic::AtomicUsize, Arc},
    task::{Context, Poll},
};
use tokio::{
    io::{reader_stream, AsyncWriteExt},
    net::UnixStream,
    stream::StreamExt,
    sync::{mpsc, oneshot},
};

/// Unix Domain Sockets (IPC) transport.
#[derive(Debug, Clone)]
pub struct Ipc {
    id: Arc<AtomicUsize>,
    messages_tx: mpsc::UnboundedSender<TransportMessage>,
}

#[cfg(unix)]
impl Ipc {
    /// Creates a new IPC transport from a given path.
    ///
    /// IPC is only available on Unix.
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let stream = UnixStream::connect(path).await?;

        Ok(Self::with_stream(stream))
    }

    fn with_stream(stream: UnixStream) -> Self {
        let id = Arc::new(AtomicUsize::new(1));
        let (messages_tx, messages_rx) = mpsc::unbounded_channel();

        tokio::spawn(run_server(stream, messages_rx));

        Ipc { id, messages_tx }
    }
}

impl Transport for Ipc {
    type Out = SingleResponse;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (crate::RequestId, rpc::Call) {
        let id = self.id.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let request = helpers::build_request(id, method, params);
        (id, request)
    }

    fn send(&self, id: RequestId, call: rpc::Call) -> Self::Out {
        let (response_tx, response_rx) = oneshot::channel();
        let message = TransportMessage::Single((id, call, response_tx));

        SingleResponse(self.messages_tx.send(message).map(|()| response_rx).map_err(Into::into))
    }
}

impl BatchTransport for Ipc {
    type Batch = BatchResponse;

    fn send_batch<T: IntoIterator<Item = (RequestId, rpc::Call)>>(&self, requests: T) -> Self::Batch {
        let mut response_rxs = vec![];

        let message = TransportMessage::Batch(
            requests
                .into_iter()
                .map(|(id, call)| {
                    let (response_tx, response_rx) = oneshot::channel();
                    response_rxs.push(response_rx);

                    (id, call, response_tx)
                })
                .collect(),
        );

        BatchResponse(
            self.messages_tx
                .send(message)
                .map(|()| join_all(response_rxs))
                .map_err(Into::into),
        )
    }
}

impl DuplexTransport for Ipc {
    type NotificationStream = mpsc::UnboundedReceiver<rpc::Value>;

    fn subscribe(&self, id: SubscriptionId) -> Result<Self::NotificationStream> {
        let (tx, rx) = mpsc::unbounded_channel();
        self.messages_tx.send(TransportMessage::Subscribe(id, tx))?;
        Ok(rx)
    }

    fn unsubscribe(&self, id: SubscriptionId) -> Result<()> {
        self.messages_tx
            .send(TransportMessage::Unsubscribe(id))
            .map_err(Into::into)
    }
}

/// A future representing a pending RPC request. Resolves to a JSON RPC value.
pub struct SingleResponse(Result<oneshot::Receiver<rpc::Value>>);

impl futures::Future for SingleResponse {
    type Output = Result<rpc::Value>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.0 {
            Err(err) => Poll::Ready(Err(err.clone())),
            Ok(ref mut rx) => {
                let value = ready!(futures::Future::poll(Pin::new(rx), cx))?;
                Poll::Ready(Ok(value))
            }
        }
    }
}

/// A future representing a pending batch RPC request. Resolves to a vector of JSON RPC value.
pub struct BatchResponse(Result<JoinAll<oneshot::Receiver<rpc::Value>>>);

impl futures::Future for BatchResponse {
    type Output = Result<Vec<Result<rpc::Value>>>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match &mut self.0 {
            Err(err) => Poll::Ready(Err(err.clone())),
            Ok(ref mut rxs) => {
                let poll = futures::Future::poll(Pin::new(rxs), cx);
                let values = ready!(poll).into_iter().map(|r| r.map_err(Into::into)).collect();

                Poll::Ready(Ok(values))
            }
        }
    }
}

type TransportRequest = (RequestId, rpc::Call, oneshot::Sender<rpc::Value>);

#[derive(Debug)]
enum TransportMessage {
    Single(TransportRequest),
    Batch(Vec<TransportRequest>),
    Subscribe(SubscriptionId, mpsc::UnboundedSender<rpc::Value>),
    Unsubscribe(SubscriptionId),
}

#[cfg(unix)]
async fn run_server(mut unix_stream: UnixStream, messages_rx: mpsc::UnboundedReceiver<TransportMessage>) -> Result<()> {
    let (socket_reader, mut socket_writer) = unix_stream.split();
    let mut pending_response_txs = BTreeMap::default();
    let mut subscription_txs = BTreeMap::default();

    let mut socket_reader = reader_stream(socket_reader);
    let mut messages_rx = messages_rx.fuse();
    let mut read_buffer = vec![];
    let mut closed = false;

    while !closed || pending_response_txs.len() > 0 {
        tokio::select! {
            message = messages_rx.next() => match message {
                None => closed = true,
                Some(TransportMessage::Subscribe(id, tx)) => {
                    if let Some(_) = subscription_txs.insert(id.clone(), tx) {
                        log::warn!("Replacing a subscription with id {:?}", id);
                    }
                },
                Some(TransportMessage::Unsubscribe(id)) => {
                    if let None = subscription_txs.remove(&id) {
                        log::warn!("Unsubscribing not subscribed id {:?}", id);
                    }
                },
                Some(TransportMessage::Single((request_id, rpc_call, response_tx))) => {
                    if pending_response_txs.insert(request_id, response_tx).is_some() {
                        log::warn!("Replacing a pending request with id {:?}", request_id);
                    }

                    let bytes = helpers::to_string(&rpc::Request::Single(rpc_call)).into_bytes();
                    if let Err(err) = socket_writer.write(&bytes).await {
                        pending_response_txs.remove(&request_id);
                        log::error!("IPC write error: {:?}", err);
                    }
                }
                Some(TransportMessage::Batch(requests)) => {
                    let mut request_ids = vec![];
                    let mut rpc_calls = vec![];

                    for (request_id, rpc_call, response_tx) in requests {
                        request_ids.push(request_id);
                        rpc_calls.push(rpc_call);

                        if pending_response_txs.insert(request_id, response_tx).is_some() {
                            log::warn!("Replacing a pending request with id {:?}", request_id);
                        }
                    }

                    let bytes = helpers::to_string(&rpc::Request::Batch(rpc_calls)).into_bytes();

                    if let Err(err) = socket_writer.write(&bytes).await {
                        log::error!("IPC write error: {:?}", err);
                        for request_id in request_ids {
                            pending_response_txs.remove(&request_id);
                        }
                    }
                }
            },
            bytes = socket_reader.next() => match bytes {
                Some(Ok(bytes)) => {
                    read_buffer.extend_from_slice(&bytes);

                    let read_len = {
                        let mut de: serde_json::StreamDeserializer<_, serde_json::Value> =
                            serde_json::Deserializer::from_slice(&read_buffer).into_iter();

                        while let Some(Ok(value)) = de.next() {
                            if let Ok(notification) = serde_json::from_value::<rpc::Notification>(value.clone()) {
                                let _ = notify(&mut subscription_txs, notification);
                                continue;
                            }

                            if let Ok(response) = serde_json::from_value::<rpc::Response>(value) {
                                let _ = respond(&mut pending_response_txs, response);
                                continue;
                            }

                            log::warn!("JSON is not a response or notification");
                        }

                        de.byte_offset()
                    };

                    read_buffer.copy_within(read_len.., 0);
                    read_buffer.truncate(read_buffer.len() - read_len);
                },
                Some(Err(err)) => {
                    log::error!("IPC read error: {:?}", err);
                    return Err(err.into());
                },
                None => break,
            }
        };
    }

    Ok(())
}

fn notify(
    subscription_txs: &mut BTreeMap<SubscriptionId, mpsc::UnboundedSender<rpc::Value>>,
    notification: rpc::Notification,
) -> std::result::Result<(), ()> {
    if let rpc::Params::Map(params) = notification.params {
        let id = params.get("subscription");
        let result = params.get("result");

        if let (Some(&rpc::Value::String(ref id)), Some(result)) = (id, result) {
            let id: SubscriptionId = id.clone().into();
            if let Some(tx) = subscription_txs.get(&id) {
                if let Err(e) = tx.send(result.clone()) {
                    log::error!("Error sending notification: {:?} (id: {:?}", e, id);
                }
            } else {
                log::warn!("Got notification for unknown subscription (id: {:?})", id);
            }
        } else {
            log::error!("Got unsupported notification (id: {:?})", id);
        }
    }

    Ok(())
}

fn respond(
    pending_response_txs: &mut BTreeMap<RequestId, oneshot::Sender<rpc::Value>>,
    response: rpc::Response,
) -> std::result::Result<(), ()> {
    let outputs = match response {
        rpc::Response::Single(output) => vec![output],
        rpc::Response::Batch(outputs) => outputs,
    };

    for output in outputs {
        let _ = respond_output(pending_response_txs, output);
    }

    Ok(())
}

fn respond_output(
    pending_response_txs: &mut BTreeMap<RequestId, oneshot::Sender<rpc::Value>>,
    output: rpc::Output,
) -> std::result::Result<(), ()> {
    let id = output.id().clone();

    let value = helpers::to_result_from_output(output).map_err(|err| {
        log::warn!("Unable to parse output into rpc::Value: {:?}", err);
    })?;

    let id = match id {
        rpc::Id::Num(num) => num as usize,
        _ => {
            log::warn!("Got unsupported response (id: {:?})", id);
            return Err(());
        }
    };

    let response_tx = pending_response_txs.remove(&id).ok_or_else(|| {
        log::warn!("Got response for unknown request (id: {:?})", id);
    })?;

    response_tx.send(value).map_err(|err| {
        log::warn!("Sending a response to deallocated channel: {:?}", err);
    })
}

impl From<mpsc::error::SendError<TransportMessage>> for Error {
    fn from(err: mpsc::error::SendError<TransportMessage>) -> Self {
        Error::Transport(format!("Send Error: {:?}", err))
    }
}

impl From<oneshot::error::RecvError> for Error {
    fn from(err: oneshot::error::RecvError) -> Self {
        Error::Transport(format!("Recv Error: {:?}", err))
    }
}

#[cfg(all(test, unix))]
mod test {
    use super::*;
    use serde_json::json;
    use tokio::{
        io::{reader_stream, AsyncWriteExt},
        net::UnixStream,
    };

    #[tokio::test]
    async fn works_for_single_requests() {
        let (stream1, stream2) = UnixStream::pair().unwrap();
        let ipc = Ipc::with_stream(stream1);

        tokio::spawn(eth_node_single(stream2));

        let (req_id, request) = ipc.prepare(
            "eth_test",
            vec![json!({
                "test": -1,
            })],
        );
        let response = ipc.send(req_id, request).await;
        let expected_response_json: serde_json::Value = json!({
            "test": 1,
        });
        assert_eq!(response, Ok(expected_response_json));

        let (req_id, request) = ipc.prepare(
            "eth_test",
            vec![json!({
                "test": 3,
            })],
        );
        let response = ipc.send(req_id, request).await;
        let expected_response_json: serde_json::Value = json!({
            "test": "string1",
        });
        assert_eq!(response, Ok(expected_response_json));
    }

    async fn eth_node_single(stream: UnixStream) {
        let (rx, mut tx) = stream.into_split();

        let mut rx = reader_stream(rx);
        if let Some(Ok(bytes)) = rx.next().await {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

            assert_eq!(
                v,
                json!({
                    "jsonrpc": "2.0",
                    "method": "eth_test",
                    "id": 1,
                    "params": [{
                        "test": -1
                    }]
                })
            );

            tx.write(r#"{"jsonrpc": "2.0", "id": 1, "result": {"test": 1}}"#.as_ref())
                .await
                .unwrap();
            tx.flush().await.unwrap();
        }

        if let Some(Ok(bytes)) = rx.next().await {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

            assert_eq!(
                v,
                json!({
                    "jsonrpc": "2.0",
                    "method": "eth_test",
                    "id": 2,
                    "params": [{
                        "test": 3
                    }]
                })
            );

            let response_bytes = r#"{"jsonrpc": "2.0", "id": 2, "result": {"test": "string1"}}"#;
            for chunk in response_bytes.as_bytes().chunks(3) {
                tx.write(chunk).await.unwrap();
                tx.flush().await.unwrap();
            }
        }
    }

    #[tokio::test]
    async fn works_for_batch_request() {
        let (stream1, stream2) = UnixStream::pair().unwrap();
        let ipc = Ipc::with_stream(stream1);

        tokio::spawn(eth_node_batch(stream2));

        let requests = vec![json!({"test": -1,}), json!({"test": 3,})];
        let requests = requests.into_iter().map(|v| ipc.prepare("eth_test", vec![v]));

        let response = ipc.send_batch(requests).await;
        let expected_response_json = vec![Ok(json!({"test": 1})), Ok(json!({"test": "string1"}))];

        assert_eq!(response, Ok(expected_response_json));
    }

    async fn eth_node_batch(stream: UnixStream) {
        let (rx, mut tx) = stream.into_split();

        let mut rx = reader_stream(rx);
        if let Some(Ok(bytes)) = rx.next().await {
            let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

            assert_eq!(
                v,
                json!([{
                    "jsonrpc": "2.0",
                    "method": "eth_test",
                    "id": 1,
                    "params": [{
                        "test": -1
                    }]
                }, {
                    "jsonrpc": "2.0",
                    "method": "eth_test",
                    "id": 2,
                    "params": [{
                        "test": 3
                    }]
                }])
            );

            let response = json!([
                {"jsonrpc": "2.0", "id": 1, "result": {"test": 1}},
                {"jsonrpc": "2.0", "id": 2, "result": {"test": "string1"}},
            ]);

            tx.write_all(serde_json::to_string(&response).unwrap().as_ref())
                .await
                .unwrap();

            tx.flush().await.unwrap();
        }
    }

    #[tokio::test]
    async fn works_for_partial_batches() {
        let (stream1, stream2) = UnixStream::pair().unwrap();
        let ipc = Ipc::with_stream(stream1);

        tokio::spawn(eth_node_partial_batches(stream2));

        let requests = vec![json!({"test": 0}), json!({"test": 1}), json!({"test": 2})];
        let requests = requests.into_iter().map(|v| ipc.execute("eth_test", vec![v]));
        let responses = join_all(requests).await;

        assert_eq!(responses[0], Ok(json!({"test": 0})));
        assert_eq!(responses[2], Ok(json!({"test": 2})));
        assert!(responses[1].is_err());
    }

    async fn eth_node_partial_batches(stream: UnixStream) {
        let (rx, mut tx) = stream.into_split();
        let mut buf = vec![];
        let mut rx = reader_stream(rx);
        while let Some(Ok(bytes)) = rx.next().await {
            buf.extend(bytes);

            let requests: std::result::Result<Vec<serde_json::Value>, serde_json::Error> =
                serde_json::Deserializer::from_slice(&buf).into_iter().collect();

            if let Ok(requests) = requests {
                if requests.len() == 3 {
                    break;
                }
            }
        }

        let response = json!([
            {"jsonrpc": "2.0", "id": 1, "result": {"test": 0}},
            {"jsonrpc": "2.0", "id": "2", "result": {"test": 2}},
            {"jsonrpc": "2.0", "id": 3, "result": {"test": 2}},
        ]);

        tx.write_all(serde_json::to_string(&response).unwrap().as_ref())
            .await
            .unwrap();

        tx.flush().await.unwrap();
    }
}
