//! HTTP Transport

use crate::{
    error::{Error, Result, TransportError},
    helpers, BatchTransport, RequestId, Transport,
};
#[cfg(not(feature = "wasm"))]
use futures::future::BoxFuture;
#[cfg(feature = "wasm")]
use futures::future::LocalBoxFuture as BoxFuture;
use jsonrpc_core::types::{Call, Output, Request, Value};
use reqwest::{Client, Url};
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

/// HTTP Transport
#[derive(Clone, Debug)]
pub struct Http {
    // Client is already an Arc so doesn't need to be part of inner.
    client: Client,
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    url: Url,
    id: AtomicUsize,
}

impl Http {
    /// Create new HTTP transport connecting to given URL.
    ///
    /// Note that the http [Client] automatically enables some features like setting the basic auth
    /// header or enabling a proxy from the environment. You can customize it with
    /// [Http::with_client].
    pub fn new(url: &str) -> Result<Self> {
        #[allow(unused_mut)]
        let mut builder = Client::builder();
        #[cfg(not(feature = "wasm"))]
        {
            builder = builder.user_agent(reqwest::header::HeaderValue::from_static("web3.rs"));
        }
        let client = builder
            .build()
            .map_err(|err| Error::Transport(TransportError::Message(format!("failed to build client: {}", err))))?;
        Ok(Self::with_client(client, url.parse()?))
    }

    /// Like `new` but with a user provided client instance.
    pub fn with_client(client: Client, url: Url) -> Self {
        Self {
            client,
            inner: Arc::new(Inner {
                url,
                id: AtomicUsize::new(0),
            }),
        }
    }

    fn next_id(&self) -> RequestId {
        self.inner.id.fetch_add(1, Ordering::AcqRel)
    }

    fn new_request(&self) -> (Client, Url) {
        (self.client.clone(), self.inner.url.clone())
    }
}

// Id is only used for logging.
async fn execute_rpc<T: DeserializeOwned>(client: &Client, url: Url, request: &Request, id: RequestId) -> Result<T> {
    log::debug!("[id:{}] sending request: {:?}", id, serde_json::to_string(&request)?);
    let response = client
        .post(url)
        .json(request)
        .send()
        .await
        .map_err(|err| Error::Transport(TransportError::Message(format!("failed to send request: {}", err))))?;
    let status = response.status();
    let response = response.bytes().await.map_err(|err| {
        Error::Transport(TransportError::Message(format!(
            "failed to read response bytes: {}",
            err
        )))
    })?;
    log::debug!(
        "[id:{}] received response: {:?}",
        id,
        String::from_utf8_lossy(&response)
    );
    if !status.is_success() {
        return Err(Error::Transport(TransportError::Code(status.as_u16())));
    }
    helpers::arbitrary_precision_deserialize_workaround(&response).map_err(|err| {
        Error::Transport(TransportError::Message(format!(
            "failed to deserialize response: {}: {}",
            err,
            String::from_utf8_lossy(&response)
        )))
    })
}

type RpcResult = Result<Value>;

impl Transport for Http {
    type Out = BoxFuture<'static, Result<Value>>;

    fn prepare(&self, method: &str, params: Vec<Value>) -> (RequestId, Call) {
        let id = self.next_id();
        let request = helpers::build_request(id, method, params);
        (id, request)
    }

    fn send(&self, id: RequestId, call: Call) -> Self::Out {
        let (client, url) = self.new_request();
        Box::pin(async move {
            let output: Output = execute_rpc(&client, url, &Request::Single(call), id).await?;
            helpers::to_result_from_output(output)
        })
    }
}

impl BatchTransport for Http {
    type Batch = BoxFuture<'static, Result<Vec<RpcResult>>>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, Call)>,
    {
        // Batch calls don't need an id but it helps associate the response log with the request log.
        let id = self.next_id();
        let (client, url) = self.new_request();
        let (ids, calls): (Vec<_>, Vec<_>) = requests.into_iter().unzip();
        Box::pin(async move {
            let value = execute_rpc(&client, url, &Request::Batch(calls), id).await?;
            let outputs = handle_possible_error_object_for_batched_request(value)?;
            handle_batch_response(&ids, outputs)
        })
    }
}

fn handle_possible_error_object_for_batched_request(value: Value) -> Result<Vec<Output>> {
    if value.is_object() {
        let output: Output = serde_json::from_value(value)?;
        return Err(match output {
            Output::Failure(failure) => Error::Rpc(failure.error),
            Output::Success(success) => {
                // totally unlikely - we got json success object for batched request
                Error::InvalidResponse(format!("Invalid response for batched request: {:?}", success))
            }
        });
    }
    let outputs = serde_json::from_value(value)?;
    Ok(outputs)
}

// According to the jsonrpc specification batch responses can be returned in any order so we need to
// restore the intended order.
fn handle_batch_response(ids: &[RequestId], outputs: Vec<Output>) -> Result<Vec<RpcResult>> {
    if ids.len() != outputs.len() {
        return Err(Error::InvalidResponse("unexpected number of responses".to_string()));
    }
    let mut outputs = outputs
        .into_iter()
        .map(|output| Ok((id_of_output(&output)?, helpers::to_result_from_output(output))))
        .collect::<Result<HashMap<_, _>>>()?;
    ids.iter()
        .map(|id| {
            outputs
                .remove(id)
                .ok_or_else(|| Error::InvalidResponse(format!("batch response is missing id {}", id)))
        })
        .collect()
}

fn id_of_output(output: &Output) -> Result<RequestId> {
    let id = match output {
        Output::Success(success) => &success.id,
        Output::Failure(failure) => &failure.id,
    };
    match id {
        jsonrpc_core::Id::Num(num) => Ok(*num as RequestId),
        _ => Err(Error::InvalidResponse("response id is not u64".to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error::Rpc;
    use jsonrpc_core::ErrorCode;
    use std::net::TcpListener;

    fn get_available_port() -> Option<u16> {
        Some(TcpListener::bind(("127.0.0.1", 0)).ok()?.local_addr().ok()?.port())
    }

    async fn server(req: hyper::Request<hyper::Body>) -> hyper::Result<hyper::Response<hyper::Body>> {
        use hyper::body::HttpBody;

        let expected = r#"{"jsonrpc":"2.0","method":"eth_getAccounts","params":[],"id":0}"#;
        let response = r#"{"jsonrpc":"2.0","id":0,"result":"x"}"#;

        assert_eq!(req.method(), &hyper::Method::POST);
        assert_eq!(req.uri().path(), "/");
        let mut content: Vec<u8> = vec![];
        let mut body = req.into_body();
        while let Some(Ok(chunk)) = body.data().await {
            content.extend(&*chunk);
        }
        assert_eq!(std::str::from_utf8(&*content), Ok(expected));

        Ok(hyper::Response::new(response.into()))
    }

    #[tokio::test]
    async fn should_make_a_request() {
        use hyper::service::{make_service_fn, service_fn};

        // given
        let addr = format!("127.0.0.1:{}", get_available_port().unwrap());
        // start server
        let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(server)) });
        let server = hyper::Server::bind(&addr.parse().unwrap()).serve(service);
        let addr_clone = addr.clone();
        tokio::spawn(async move {
            println!("Listening on http://{}", addr_clone);
            server.await.unwrap();
        });

        // when
        let client = Http::new(&format!("http://{}", &addr)).unwrap();
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        println!("Got response");

        // then
        assert_eq!(response, Ok(Value::String("x".into())));
    }

    #[tokio::test]
    async fn catch_generic_json_error_for_batched_request() {
        use hyper::service::{make_service_fn, service_fn};

        async fn handler(_req: hyper::Request<hyper::Body>) -> hyper::Result<hyper::Response<hyper::Body>> {
            let response = r#"{
                "jsonrpc":"2.0",
                "error":{
                    "code":0,
                    "message":"we can't execute this request"
                },
                "id":null
            }"#;
            Ok(hyper::Response::<hyper::Body>::new(response.into()))
        }

        // given
        let addr = format!("127.0.0.1:{}", get_available_port().unwrap());
        // start server
        let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(handler)) });
        let server = hyper::Server::bind(&addr.parse().unwrap()).serve(service);
        let addr_clone = addr.clone();
        tokio::spawn(async move {
            println!("Listening on http://{}", addr_clone);
            server.await.unwrap();
        });

        // when
        let client = Http::new(&format!("http://{}", &addr)).unwrap();
        println!("Sending request");
        let response = client
            .send_batch(vec![client.prepare("some_method", vec![])].into_iter())
            .await;
        println!("Got response");

        // then
        assert_eq!(
            response,
            Err(Rpc(crate::rpc::error::Error {
                code: ErrorCode::ServerError(0),
                message: "we can't execute this request".to_string(),
                data: None,
            }))
        );
    }

    #[test]
    fn handles_batch_response_being_in_different_order_than_input() {
        let ids = vec![0, 1, 2];
        // This order is different from the ids.
        let outputs = [1u64, 0, 2]
            .iter()
            .map(|&id| {
                Output::Success(jsonrpc_core::Success {
                    jsonrpc: None,
                    result: id.into(),
                    id: jsonrpc_core::Id::Num(id),
                })
            })
            .collect();
        let results = handle_batch_response(&ids, outputs)
            .unwrap()
            .into_iter()
            .map(|result| result.unwrap().as_u64().unwrap() as usize)
            .collect::<Vec<_>>();
        // The order of the ids should have been restored.
        assert_eq!(ids, results);
    }
}
