//! HTTP Transport

use crate::{
    error::{Error, RateLimit, Result, TransportError},
    helpers, BatchTransport, RequestId, Transport,
};
use chrono::{DateTime, Utc};
use core::time::Duration;
#[cfg(not(feature = "wasm"))]
use futures::future::BoxFuture;
#[cfg(feature = "wasm")]
use futures::future::LocalBoxFuture as BoxFuture;
use futures_timer::Delay;
use jsonrpc_core::types::{Call, Output, Request, Value};
use reqwest::{header::HeaderMap, Client, Url};
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
    url: Url,
    client: Client,
    retries: Retries,
    inner: Arc<Inner>,
}

#[derive(Debug)]
struct Inner {
    id: AtomicUsize,
}

/// Configures retries on rate limited and failed requests.
#[derive(Debug, Clone, Default)]
pub struct Retries {
    /// Retries failed request X times when encountering 429 or 500+ status codes.
    pub max_retries: u32,

    /// Retries failed request after X time when encountering 429 or 500+ status codes.
    pub sleep_for: Duration,

    /// Retries rate limited request after time interval specified in the `Retry-After` header instead of specified sleep duration when encountering 429 status code.
    pub use_retry_after_header: bool,
}

impl Retries {
    fn step(&self) -> Self {
        Self {
            max_retries: 0.max(self.max_retries - 1),
            sleep_for: self.sleep_for * 2,
            use_retry_after_header: self.use_retry_after_header,
        }
    }
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
        Self::with_retries(client, url, Retries::default())
    }

    /// Creates client with provided client and user configured retries.
    pub fn with_retries(client: Client, url: Url, retries: Retries) -> Self {
        Self {
            client,
            url,
            retries,
            inner: Arc::new(Inner {
                id: AtomicUsize::new(0),
            }),
        }
    }

    fn next_id(&self) -> RequestId {
        self.inner.id.fetch_add(1, Ordering::AcqRel)
    }

    fn new_request(&self) -> (Client, Url, Retries) {
        (self.client.clone(), self.url.clone(), self.retries.clone())
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
    let headers = response.headers().clone();
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
    if status.as_u16() == 429 {
        let error = match extract_retry_after_value(&headers) {
            DelayAfter::Seconds(seconds) => TransportError::RateLimit(RateLimit::Seconds(seconds)),
            DelayAfter::Date(date) => TransportError::RateLimit(RateLimit::Date(date)),
            DelayAfter::None => TransportError::Code(status.as_u16()),
        };
        return Err(Error::Transport(error));
    }
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

#[derive(Debug)]
enum DelayAfter {
    Seconds(u64),
    Date(String),
    None,
}

fn extract_retry_after_value(headers: &HeaderMap) -> DelayAfter {
    let Some(header) = headers.get("Retry-After").and_then(|header| header.to_str().ok()) else {
        return DelayAfter::None;
    };

    if let Ok(seconds) = header.parse::<u64>() {
        return DelayAfter::Seconds(seconds);
    }

    DelayAfter::Date(header.to_string())
}

fn execute_rpc_with_retries<'a, T: DeserializeOwned + std::marker::Send>(
    client: &'a Client,
    url: Url,
    request: &'a Request,
    id: RequestId,
    retries: Retries,
) -> BoxFuture<'a, Result<T>> {
    Box::pin(async move {
        match execute_rpc(client, url.clone(), request, id).await {
            Ok(output) => Ok(output),
            Err(Error::Transport(error)) => match error {
                TransportError::Code(code) => {
                    if retries.max_retries <= 0
                        || retries.sleep_for <= Duration::from_secs(0)
                        || (code != 429 && code < 500)
                    {
                        return Err(Error::Transport(error));
                    }

                    Delay::new(retries.sleep_for).await;
                    execute_rpc_with_retries(client, url, request, id, retries.step()).await
                }
                TransportError::Message(message) => Err(Error::Transport(TransportError::Message(message))),
                TransportError::RateLimit(limit) => {
                    if !retries.use_retry_after_header && retries.max_retries <= 0 {
                        return Err(Error::Transport(TransportError::Code(429)));
                    }

                    match limit {
                        RateLimit::Date(date) => {
                            let Ok(until) = DateTime::parse_from_rfc2822(&date) else {
                                return Err(Error::Transport(TransportError::Code(429)));
                            };

                            let from_now = until.with_timezone(&Utc::now().timezone()) - Utc::now();
                            let secs = from_now.num_seconds() + 1; // +1 for rounding
                            if secs > 0 {
                                Delay::new(Duration::from_secs(secs as u64)).await;
                            }

                            execute_rpc_with_retries(client, url, request, id, retries.step()).await
                        }
                        RateLimit::Seconds(seconds) => {
                            Delay::new(Duration::from_secs(seconds)).await;
                            execute_rpc_with_retries(client, url, request, id, retries.step()).await
                        }
                    }
                }
            },
            Err(err) => Err(err),
        }
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
        let (client, url, retries) = self.new_request();
        Box::pin(async move {
            let output: Output = execute_rpc_with_retries(&client, url, &Request::Single(call), id, retries).await?;
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
        let (client, url, retries) = self.new_request();
        let (ids, calls): (Vec<_>, Vec<_>) = requests.into_iter().unzip();
        Box::pin(async move {
            let value = execute_rpc_with_retries(&client, url, &Request::Batch(calls), id, retries).await?;
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
    use std::{future::Future, pin::Pin};

    use super::*;
    use crate::Error::Rpc;
    use futures::lock::Mutex;
    use http_body_util::{BodyExt, Full};
    use hyper::{
        body::{Bytes, Incoming},
        server::conn::http1,
        service::service_fn,
        Method, Request, Response,
    };
    use hyper_util::rt::TokioIo;
    use jsonrpc_core::ErrorCode;
    use tokio::{net::TcpListener, task::JoinHandle, time::Instant};

    type HyperResponse =
        Pin<Box<dyn Future<Output = std::result::Result<Response<Full<Bytes>>, hyper::http::Error>> + Send + Sync>>;

    type HyperHandler = Box<dyn Fn(Request<Incoming>) -> HyperResponse + Send + Sync>;

    async fn get_available_port() -> Option<u16> {
        Some(
            TcpListener::bind(("127.0.0.1", 0))
                .await
                .ok()?
                .local_addr()
                .ok()?
                .port(),
        )
    }

    async fn create_server(port: u16, handler: HyperHandler) -> JoinHandle<()> {
        let addr = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(addr).await.unwrap();
        let handler = Arc::new(handler);
        tokio::task::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let service = service_fn(handler.as_ref());
                let io = TokioIo::new(stream);
                http1::Builder::new().serve_connection(io, service).await.unwrap();
            }
        })
    }

    fn create_client(port: u16, retries: Retries) -> Http {
        Http::with_retries(
            Client::new(),
            format!("http://127.0.0.1:{}", port).parse().unwrap(),
            retries,
        )
    }

    fn return_429(retry_after_value: Option<String>) -> HyperHandler {
        Box::new(move |_req: Request<Incoming>| -> HyperResponse {
            let retry_after_value = retry_after_value.clone();
            let response_body = Bytes::from(
                r#"{
                "jsonrpc": "2.0",
                "error": {
                    "code": 429,
                    "message": "Your app has exceeded its compute units per second capacity."
                }
            }"#,
            );

            let mut response = Response::builder().status(hyper::StatusCode::TOO_MANY_REQUESTS);
            if let Some(value) = retry_after_value {
                response = response.header("Retry-After", value)
            }

            let response = response.body(Full::new(response_body)).unwrap();
            Box::pin(async move { Ok(response) })
        })
    }

    fn return_5xx(code: u16) -> HyperHandler {
        Box::new(move |_req: Request<Incoming>| -> HyperResponse {
            let response_body = Bytes::from(
                r#"{
                "jsonrpc": "2.0",
                "error": {
                    "code": 500,
                    "message": "We can't execute this request"
                }
            }"#,
            );

            let response = Response::builder().status(code).body(Full::new(response_body)).unwrap();
            Box::pin(async move { Ok(response) })
        })
    }

    fn check_and_return_mock_response(req: Request<Incoming>) -> HyperResponse {
        let expected = r#"{"jsonrpc":"2.0","method":"eth_getAccounts","params":[],"id":0}"#;
        let response = r#"{"jsonrpc":"2.0","id":0,"result":"x"}"#;

        assert_eq!(req.method(), &Method::POST);
        assert_eq!(req.uri().path(), "/");
        let mut content: Vec<u8> = vec![];
        let mut body = req.into_body();

        Box::pin(async move {
            while let Some(Ok(chunk)) = body.frame().await {
                content.extend(chunk.into_data().unwrap());
            }
            assert_eq!(std::str::from_utf8(&content), Ok(expected));
            Response::builder().status(200).body(Full::new(response.into()))
        })
    }

    fn return_error_response(_req: Request<Incoming>) -> HyperResponse {
        let response = r#"{
            "jsonrpc":"2.0",
            "error":{
                "code":0,
                "message":"we can't execute this request"
            },
            "id":null
        }"#;
        let response = Response::builder()
            .status(200)
            .body(Full::new(response.into()))
            .unwrap();
        Box::pin(async move { Ok(response) })
    }

    fn return_sequence(handlers: Vec<HyperHandler>) -> HyperHandler {
        let handlers = Arc::new(Mutex::new(handlers));
        Box::new(move |_req: Request<Incoming>| -> HyperResponse {
            let handlers = handlers.clone();
            Box::pin(async move {
                let mut handlers = handlers.lock().await;
                let handler = handlers.remove(0);
                handler(_req).await
            })
        })
    }

    #[tokio::test]
    async fn should_make_a_request() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(port, Box::new(check_and_return_mock_response)).await;
        let client = create_client(port, Retries::default());

        // when
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        println!("Got response");

        // then
        assert_eq!(response, Ok(Value::String("x".into())));
    }

    #[tokio::test]
    async fn catch_generic_json_error_for_batched_request() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(port, Box::new(return_error_response)).await;
        let client = create_client(port, Retries::default());

        // when
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

    #[tokio::test]
    async fn status_code_429_with_retry_after_as_seconds() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(
            port,
            return_sequence(vec![
                return_429(Some("3".into())),
                Box::new(check_and_return_mock_response),
            ]),
        )
        .await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: true,
                max_retries: 3,
                ..Default::default()
            },
        );

        // when
        println!("Sending request");
        let started = Instant::now();
        let response = client.execute("eth_getAccounts", vec![]).await;
        let finished = Instant::now();
        println!("Got response");

        // then
        assert_eq!(response, Ok(Value::String("x".into())));
        assert!(finished - started >= Duration::from_secs(3));
    }

    #[tokio::test]
    async fn status_code_429_with_retry_after_as_date() {
        // given
        let port = get_available_port().await.unwrap();
        let started = Instant::now();
        let retry_after_value: DateTime<Utc> = DateTime::from(Utc::now() + Duration::from_secs(3));
        let _ = create_server(
            port,
            return_sequence(vec![
                return_429(Some(retry_after_value.to_rfc2822())),
                Box::new(check_and_return_mock_response),
            ]),
        )
        .await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: true,
                max_retries: 3,
                ..Default::default()
            },
        );

        // when
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        let finished = Instant::now();
        println!("Got response");

        // then
        assert_eq!(response, Ok(Value::String("x".into())));
        assert!(finished - started >= Duration::from_secs(3));
    }

    #[tokio::test]
    async fn status_code_429_with_invalid_retry_after() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(
            port,
            return_sequence(vec![return_429(Some("retry some time later, idc".into()))]),
        )
        .await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: true,
                max_retries: 3,
                ..Default::default()
            },
        );

        // when
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        println!("Got response");

        // then
        assert_eq!(response, Err(crate::Error::Transport(TransportError::Code(429))));
    }

    #[tokio::test]
    async fn status_code_429_without_retry_after() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(port, return_sequence(vec![return_429(None)])).await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: true,
                max_retries: 3,
                ..Default::default()
            },
        );

        // when
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        println!("Got response");

        // then
        assert_eq!(response, Err(crate::Error::Transport(TransportError::Code(429))));
    }

    #[tokio::test]
    async fn status_code_429_retry_after_disabled() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(port, return_sequence(vec![return_429(Some("3".into()))])).await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: false,
                max_retries: 0,
                sleep_for: Duration::from_secs(1),
            },
        );

        // when
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        println!("Got response");

        // then
        assert_eq!(response, Err(crate::Error::Transport(TransportError::Code(429))));
    }

    #[tokio::test]
    async fn status_code_429_with_retries() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(
            port,
            return_sequence(vec![
                return_429(Some("3".into())), // sleep for 1 second as configured below
                return_429(Some("3".into())), // sleep for 2 seconds (2x 1sec)
                Box::new(check_and_return_mock_response),
            ]),
        )
        .await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: false,
                max_retries: 3,
                sleep_for: Duration::from_secs(1),
            },
        );

        // when
        println!("Sending request");
        let started = Instant::now();
        let response = client.execute("eth_getAccounts", vec![]).await;
        let finished = Instant::now();
        println!("Got response");

        // then
        assert_eq!(response, Ok(Value::String("x".into())));
        assert!(finished - started >= Duration::from_secs(3));
    }

    #[tokio::test]
    async fn status_code_5xx_with_retries() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(
            port,
            return_sequence(vec![
                return_5xx(500), // sleep for 1 second as configured below
                return_5xx(502), // sleep for 2 seconds (2x 1sec)
                Box::new(check_and_return_mock_response),
            ]),
        )
        .await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: false,
                max_retries: 3,
                sleep_for: Duration::from_secs(1),
            },
        );

        // when
        println!("Sending request");
        let started = Instant::now();
        let response = client.execute("eth_getAccounts", vec![]).await;
        let finished = Instant::now();
        println!("Got response");

        // then
        assert_eq!(response, Ok(Value::String("x".into())));
        assert!(finished - started >= Duration::from_secs(3));
    }

    #[tokio::test]
    async fn status_code_5xx_retries_exhausted() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(
            port,
            return_sequence(vec![
                return_5xx(500), // sleep for 1 second as configured below
                return_5xx(502), // sleep for 2 seconds (2x 1sec)
                return_5xx(503),
                Box::new(check_and_return_mock_response),
            ]),
        )
        .await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: false,
                max_retries: 2,
                sleep_for: Duration::from_secs(1),
            },
        );

        // when
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        println!("Got response");

        // then
        assert_eq!(response, Err(crate::Error::Transport(TransportError::Code(503))));
    }

    #[tokio::test]
    async fn status_code_5xx_without_retries() {
        // given
        let port = get_available_port().await.unwrap();
        let _ = create_server(port, return_sequence(vec![return_5xx(500)])).await;
        let client = create_client(
            port,
            Retries {
                use_retry_after_header: true,
                max_retries: 3,
                sleep_for: Duration::from_secs(0),
            },
        );

        // when
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        println!("Got response");

        // then
        assert_eq!(response, Err(crate::Error::Transport(TransportError::Code(500))));
    }
}
