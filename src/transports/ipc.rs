//! IPC transport

use crate::{helpers, Error, RequestId, Result, Transport};
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
    type Out = Response;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (crate::RequestId, rpc::Call) {
        let id = self.id.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
        let request = helpers::build_request(id, method, params);
        (id, request)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        let (response_tx, response_rx) = oneshot::channel();

        let message = TransportMessage {
            request: rpc::Request::Single(request),
            response_tx,
            id,
        };

        Response(self.messages_tx.send(message).map(|()| response_rx).map_err(Into::into))
    }
}

/// A future representing a pending RPC request. Resolves to a JSON RPC value.
pub struct Response(Result<oneshot::Receiver<rpc::Value>>);

impl futures::Future for Response {
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

#[derive(Debug)]
struct TransportMessage {
    id: RequestId,
    request: rpc::Request,
    response_tx: oneshot::Sender<rpc::Value>,
}

#[cfg(unix)]
async fn run_server(unix_stream: UnixStream, messages_rx: mpsc::UnboundedReceiver<TransportMessage>) -> Result<()> {
    let (socket_reader, mut socket_writer) = unix_stream.into_split();
    let mut pending_response_txs = BTreeMap::default();

    let mut socket_reader = reader_stream(socket_reader);
    let mut messages_rx = messages_rx.fuse();
    let mut read_buffer = vec![];

    loop {
        tokio::select! {
            message = messages_rx.next() => match message {
                Some(message) => {
                    if pending_response_txs.insert(message.id, message.response_tx).is_some() {
                        log::warn!("Replacing a pending request with id {:?}", message.id);
                    }

                    let bytes = helpers::to_string(&message.request).into_bytes();
                    if let Err(err) = socket_writer.write(&bytes).await {
                        pending_response_txs.remove(&message.id);
                        log::error!("IPC write error: {:?}", err);
                    }
                },
                None => {},
            },
            bytes = socket_reader.next() => match bytes {
                Some(Ok(bytes)) => {
                    read_buffer.extend_from_slice(&bytes);

                    let pos = match read_buffer.iter().rposition(|&b| b == b']' || b == b'}') {
                        Some(pos) => pos + 1,
                        None => continue,
                    };

                    match helpers::to_response_from_slice(&read_buffer[..pos]) {
                        Ok(response) => {
                            let remaining_bytes_len = read_buffer.len() - pos;
                            for i in 0..remaining_bytes_len {
                                read_buffer[i] = read_buffer[pos + i];
                            }
                            read_buffer.truncate(remaining_bytes_len);

                            let outputs = match response {
                                rpc::Response::Single(output) => vec![output],
                                rpc::Response::Batch(outputs) => outputs,
                            };

                            for output in outputs {
                                let id = match &output {
                                    rpc::Output::Success(success) => success.id.clone(),
                                    rpc::Output::Failure(failure) => failure.id.clone(),
                                };

                                let value = match helpers::to_result_from_output(output) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        log::warn!("Unable to parse output into rpc::Value: {:?}", err);
                                        continue;
                                    },
                                };

                                let id = match id {
                                    rpc::Id::Num(num) => num as usize,
                                    _ => {
                                        log::warn!("Got unsupported response (id: {:?})", id);
                                        continue;
                                    },
                                };

                                let response_tx = match pending_response_txs.remove(&id) {
                                    Some(response_tx) => response_tx,
                                    None => {
                                        log::warn!("Got response for unknown request (id: {:?})", id);
                                        continue;
                                    },
                                };

                                if let Err(err) = response_tx.send(value) {
                                    log::warn!("Sending a response to deallocated channel: {:?}", err);
                                }
                            }
                        }
                        Err(err) => log::warn!("Got bad IPC response bytes: {:?}", err),
                    };
                },
                Some(Err(err)) => {
                    log::error!("IPC read error: {:?}", err);
                    break;
                },
                None => break,
            }
        };
    }

    Ok(())
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
    async fn works() {
        let (stream1, stream2) = UnixStream::pair().unwrap();
        let ipc = Ipc::with_stream(stream1);

        tokio::spawn(eth_node(stream2));

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

    async fn eth_node(stream: UnixStream) {
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
}
