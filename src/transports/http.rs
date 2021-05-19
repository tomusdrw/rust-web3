//! HTTP Transport

use crate::{error, helpers, rpc, BatchTransport, Error, RequestId, Transport};
#[cfg(not(feature = "wasm"))]
use futures::future::BoxFuture;
#[cfg(feature = "wasm")]
use futures::future::LocalBoxFuture as BoxFuture;
use futures::{
    self,
    task::{Context, Poll},
    Future, FutureExt,
};
use reqwest::header::HeaderValue;
use std::{
    fmt,
    ops::Deref,
    pin::Pin,
    sync::{
        atomic::{self, AtomicUsize},
        Arc,
    },
};
use url::Url;

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Transport(format!("{:?}", err))
    }
}

impl From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(err: reqwest::header::InvalidHeaderValue) -> Self {
        Error::Transport(format!("{}", err))
    }
}

// The max string length of a request without transfer-encoding: chunked.
const MAX_SINGLE_CHUNK: usize = 256;

/// HTTP Transport (synchronous)
#[derive(Debug, Clone)]
pub struct Http {
    id: Arc<AtomicUsize>,
    url: reqwest::Url,
    basic_auth: Option<HeaderValue>,
    client: reqwest::Client,
}

impl Http {
    /// Create new HTTP transport connecting to given URL.
    pub fn new(url: &str) -> error::Result<Self> {
        #[allow(unused_mut)]
        let mut client_builder = reqwest::Client::builder();

        #[cfg(feature = "http-native-tls")]
        {
            client_builder = client_builder.use_native_tls();
        }

        #[cfg(feature = "http-rustls-tls")]
        {
            client_builder = client_builder.use_rustls_tls();
        }

        #[cfg(not(feature = "wasm"))]
        {
            let proxy_env = std::env::var("HTTPS_PROXY");
            client_builder = match proxy_env {
                Ok(proxy_scheme) => {
                    let proxy = reqwest::Proxy::all(proxy_scheme.as_str())?;
                    client_builder.proxy(proxy)
                }
                Err(_) => client_builder.no_proxy(),
            };
        }

        let client = client_builder.build()?;

        let basic_auth = {
            let url = Url::parse(url)?;
            let user = url.username();
            let auth = format!("{}:{}", user, url.password().unwrap_or_default());
            if &auth == ":" {
                None
            } else {
                Some(HeaderValue::from_str(&format!("Basic {}", base64::encode(&auth)))?)
            }
        };

        Ok(Http {
            id: Arc::new(AtomicUsize::new(1)),
            url: url.parse()?,
            basic_auth,
            client,
        })
    }

    fn send_request<F, O>(&self, id: RequestId, request: rpc::Request, extract: F) -> Response<F>
    where
        F: Fn(Vec<u8>) -> O,
    {
        let request = helpers::to_string(&request);
        log::debug!("[{}] Sending: {} to {}", id, request, self.url);
        let len = request.len();

        let mut request_builder = self
            .client
            .post(self.url.clone())
            .header(
                reqwest::header::CONTENT_TYPE,
                HeaderValue::from_static("application/json"),
            )
            .header(reqwest::header::USER_AGENT, HeaderValue::from_static("web3.rs"))
            .body(request);

        // Don't send chunked request
        if len < MAX_SINGLE_CHUNK {
            request_builder = request_builder.header(reqwest::header::CONTENT_LENGTH, len.to_string());
        }

        // Send basic auth header
        if let Some(ref basic_auth) = self.basic_auth {
            request_builder = request_builder.header(reqwest::header::AUTHORIZATION, basic_auth.clone());
        }

        let result = request_builder.send();
        Response::new(id, Box::pin(result), extract)
    }
}

impl Transport for Http {
    type Out = Response<fn(Vec<u8>) -> error::Result<rpc::Value>>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        let id = self.id.fetch_add(1, atomic::Ordering::AcqRel);
        let request = helpers::build_request(id, method, params);

        (id, request)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
        self.send_request(id, rpc::Request::Single(request), single_response)
    }
}

impl BatchTransport for Http {
    type Batch = Response<fn(Vec<u8>) -> error::Result<Vec<error::Result<rpc::Value>>>>;

    fn send_batch<T>(&self, requests: T) -> Self::Batch
    where
        T: IntoIterator<Item = (RequestId, rpc::Call)>,
    {
        let mut it = requests.into_iter();
        let (id, first) = it.next().map(|x| (x.0, Some(x.1))).unwrap_or_else(|| (0, None));
        let requests = first.into_iter().chain(it.map(|x| x.1)).collect();

        self.send_request(id, rpc::Request::Batch(requests), batch_response)
    }
}

/// Parse bytes RPC response into `Result`.
fn single_response<T: Deref<Target = [u8]>>(response: T) -> error::Result<rpc::Value> {
    let response =
        helpers::to_response_from_slice(&*response).map_err(|e| Error::InvalidResponse(format!("{:?}", e)))?;
    match response {
        rpc::Response::Single(output) => helpers::to_result_from_output(output),
        _ => Err(Error::InvalidResponse("Expected single, got batch.".into())),
    }
}

/// Parse bytes RPC batch response into `Result`.
fn batch_response<T: Deref<Target = [u8]>>(response: T) -> error::Result<Vec<error::Result<rpc::Value>>> {
    let response =
        helpers::to_response_from_slice(&*response).map_err(|e| Error::InvalidResponse(format!("{:?}", e)))?;
    match response {
        rpc::Response::Batch(outputs) => Ok(outputs.into_iter().map(helpers::to_result_from_output).collect()),
        _ => Err(Error::InvalidResponse("Expected batch, got single.".into())),
    }
}

type ResponseFuture = BoxFuture<'static, reqwest::Result<reqwest::Response>>;
type BodyFuture = BoxFuture<'static, reqwest::Result<bytes::Bytes>>;
enum ResponseState {
    Waiting(ResponseFuture),
    Reading(BodyFuture),
}

/// A future representing a response to a pending request.
pub struct Response<T> {
    id: RequestId,
    extract: T,
    state: ResponseState,
}

impl<T> Response<T> {
    /// Creates a new `Response`
    pub fn new(id: RequestId, response: ResponseFuture, extract: T) -> Self {
        log::trace!("[{}] Request pending.", id);
        Response {
            id,
            extract,
            state: ResponseState::Waiting(response),
        }
    }
}

// We can do this because `hyper::client::ResponseFuture: Unpin`.
impl<T> Unpin for Response<T> {}

impl<T, Out> Future for Response<T>
where
    T: Fn(Vec<u8>) -> error::Result<Out>,
    Out: fmt::Debug,
{
    type Output = error::Result<Out>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let id = self.id;
        loop {
            match self.state {
                ResponseState::Waiting(ref mut waiting) => {
                    log::trace!("[{}] Checking response.", id);
                    let response = ready!(waiting.poll_unpin(ctx))?;
                    if !response.status().is_success() {
                        return Poll::Ready(Err(Error::Transport(format!(
                            "Unexpected response status code: {}",
                            response.status()
                        ))));
                    }
                    self.state = ResponseState::Reading(Box::pin(response.bytes()));
                }
                ResponseState::Reading(ref mut body) => {
                    log::trace!("[{}] Reading body.", id);
                    let chunk = ready!(body.poll_unpin(ctx))?;
                    let response = chunk.to_vec();
                    log::trace!(
                        "[{}] Extracting result from:\n{}",
                        self.id,
                        std::str::from_utf8(&response).unwrap_or("<invalid utf8>")
                    );
                    return Poll::Ready((self.extract)(response));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn http_supports_basic_auth_with_user_and_password() {
        let http = Http::new("https://user:password@127.0.0.1:8545").unwrap();
        assert!(http.basic_auth.is_some());
        assert_eq!(
            http.basic_auth,
            Some(HeaderValue::from_static("Basic dXNlcjpwYXNzd29yZA=="))
        )
    }

    #[test]
    fn http_supports_basic_auth_with_user_no_password() {
        let http = Http::new("https://username:@127.0.0.1:8545").unwrap();
        assert!(http.basic_auth.is_some());
        assert_eq!(http.basic_auth, Some(HeaderValue::from_static("Basic dXNlcm5hbWU6")))
    }

    #[test]
    fn http_supports_basic_auth_with_only_password() {
        let http = Http::new("https://:password@127.0.0.1:8545").unwrap();
        assert!(http.basic_auth.is_some());
        assert_eq!(http.basic_auth, Some(HeaderValue::from_static("Basic OnBhc3N3b3Jk")))
    }

    async fn server(req: hyper::Request<hyper::Body>) -> hyper::Result<hyper::Response<hyper::Body>> {
        use hyper::body::HttpBody;

        let expected = r#"{"jsonrpc":"2.0","method":"eth_getAccounts","params":[],"id":1}"#;
        let response = r#"{"jsonrpc":"2.0","id":1,"result":"x"}"#;

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
        let addr = "127.0.0.1:3001";
        // start server
        let service = make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(server)) });
        let server = hyper::Server::bind(&addr.parse().unwrap()).serve(service);
        tokio::spawn(async move {
            println!("Listening on http://{}", addr);
            server.await.unwrap();
        });

        // when
        let client = Http::new(&format!("http://{}", addr)).unwrap();
        println!("Sending request");
        let response = client.execute("eth_getAccounts", vec![]).await;
        println!("Got response");

        // then
        assert_eq!(response, Ok(rpc::Value::String("x".into())));
    }
}
