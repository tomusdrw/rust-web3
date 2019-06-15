//! HTTP Transport

extern crate hyper;
extern crate url;

#[cfg(feature = "tls")]
extern crate hyper_tls;
#[cfg(feature = "tls")]
extern crate native_tls;

use std::ops::Deref;
use std::sync::atomic::{self, AtomicUsize};
use std::sync::Arc;

use self::hyper::header::HeaderValue;
use self::url::Url;
use crate::helpers;
use crate::rpc;
use crate::transports::shared::{EventLoopHandle, Response};
use crate::transports::tokio_core::reactor;
use crate::transports::Result;
use crate::{BatchTransport, Error, RequestId, Transport};
use base64;
use futures::sync::{mpsc, oneshot};
use futures::{self, future, Future, Stream};
use serde_json;

impl From<hyper::Error> for Error {
    fn from(err: hyper::Error) -> Self {
        Error::Transport(format!("{:?}", err))
    }
}

impl From<hyper::http::uri::InvalidUri> for Error {
    fn from(err: hyper::http::uri::InvalidUri) -> Self {
        Error::Transport(format!("{:?}", err))
    }
}

impl From<hyper::header::InvalidHeaderValue> for Error {
    fn from(err: hyper::header::InvalidHeaderValue) -> Self {
        Error::Transport(format!("{}", err))
    }
}

#[cfg(all(feature = "http", not(feature = "ws")))]
impl From<self::url::ParseError> for Error {
    fn from(err: self::url::ParseError) -> Self {
        Error::Transport(format!("{:?}", err))
    }
}

#[cfg(feature = "tls")]
impl From<native_tls::Error> for Error {
    fn from(err: native_tls::Error) -> Self {
        Error::Transport(format!("{:?}", err)).into()
    }
}

// The max string length of a request without transfer-encoding: chunked.
const MAX_SINGLE_CHUNK: usize = 256;
const DEFAULT_MAX_PARALLEL: usize = 64;
type Pending = oneshot::Sender<Result<hyper::Chunk>>;

/// A future representing pending HTTP request, resolves to a response.
pub type FetchTask<F> = Response<F, hyper::Chunk>;

/// HTTP Transport (synchronous)
#[derive(Debug, Clone)]
pub struct Http {
    id: Arc<AtomicUsize>,
    url: hyper::Uri,
    basic_auth: Option<HeaderValue>,
    write_sender: mpsc::UnboundedSender<(hyper::Request<hyper::Body>, Pending)>,
}

impl Http {
    /// Create new HTTP transport with given URL and spawn an event loop in a separate thread.
    /// NOTE: Dropping event loop handle will stop the transport layer!
    pub fn new(url: &str) -> Result<(EventLoopHandle, Self)> {
        Self::with_max_parallel(url, DEFAULT_MAX_PARALLEL)
    }

    /// Create new HTTP transport with given URL and spawn an event loop in a separate thread.
    /// You can set a maximal number of parallel requests using the second parameter.
    /// NOTE: Dropping event loop handle will stop the transport layer!
    pub fn with_max_parallel(url: &str, max_parallel: usize) -> Result<(EventLoopHandle, Self)> {
        let url = url.to_owned();
        EventLoopHandle::spawn(move |handle| Self::with_event_loop(&url, handle, max_parallel))
    }

    /// Create new HTTP transport with given URL and existing event loop handle.
    pub fn with_event_loop(url: &str, handle: &reactor::Handle, max_parallel: usize) -> Result<Self> {
        let (write_sender, write_receiver) = mpsc::unbounded();

        #[cfg(feature = "tls")]
        let client = hyper::Client::builder().build::<_, hyper::Body>(hyper_tls::HttpsConnector::new(4)?);

        #[cfg(not(feature = "tls"))]
        let client = hyper::Client::new();

        handle.spawn(
            write_receiver
                .map(move |(request, tx): (_, Pending)| {
                    client.request(request).then(move |response| Ok((response, tx)))
                })
                .buffer_unordered(max_parallel)
                .for_each(|(response, tx)| {
                    use futures::future::Either::{A, B};
                    let future = match response {
                        Ok(ref res) if !res.status().is_success() => A(future::err(
                            Error::Transport(format!("Unexpected response status code: {}", res.status())).into(),
                        )),
                        Ok(res) => B(res.into_body().concat2().map_err(Into::into)),
                        Err(err) => A(future::err(err.into())),
                    };
                    future.then(move |result| {
                        if let Err(err) = tx.send(result) {
                            log::warn!("Error resuming asynchronous request: {:?}", err);
                        }
                        Ok(())
                    })
                }),
        );

        let basic_auth = {
            let url = Url::parse(url)?;
            let user = url.username();

            if user.len() > 0 {
                let auth = match url.password() {
                    Some(pass) => format!("{}:{}", user, pass),
                    None => format!("{}:", user),
                };
                Some(HeaderValue::from_str(&format!("Basic {}", base64::encode(&auth)))?)
            } else {
                None
            }
        };

        Ok(Http {
            id: Default::default(),
            url: url.parse()?,
            basic_auth,
            write_sender,
        })
    }

    fn send_request<F, O>(&self, id: RequestId, request: rpc::Request, extract: F) -> FetchTask<F>
    where
        F: Fn(hyper::Chunk) -> O,
    {
        let request = helpers::to_string(&request);
        log::debug!("[{}] Sending: {} to {}", id, request, self.url);
        let len = request.len();
        let mut req = hyper::Request::new(hyper::Body::from(request));
        *req.method_mut() = hyper::Method::POST;
        *req.uri_mut() = self.url.clone();
        req.headers_mut().insert(
            hyper::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        req.headers_mut()
            .insert(hyper::header::USER_AGENT, HeaderValue::from_static("web3.rs"));

        // Don't send chunked request
        if len < MAX_SINGLE_CHUNK {
            req.headers_mut().insert(hyper::header::CONTENT_LENGTH, len.into());
        }
        // Send basic auth header
        if let Some(ref basic_auth) = self.basic_auth {
            req.headers_mut()
                .insert(hyper::header::AUTHORIZATION, basic_auth.clone());
        }
        let (tx, rx) = futures::oneshot();
        let result = self
            .write_sender
            .unbounded_send((req, tx))
            .map_err(|_| Error::Io(::std::io::ErrorKind::BrokenPipe.into()).into());

        Response::new(id, result, rx, extract)
    }
}

impl Transport for Http {
    type Out = FetchTask<fn(hyper::Chunk) -> Result<rpc::Value>>;

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
    type Batch = FetchTask<fn(hyper::Chunk) -> Result<Vec<Result<rpc::Value>>>>;

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
fn single_response<T: Deref<Target = [u8]>>(response: T) -> Result<rpc::Value> {
    let response =
        serde_json::from_slice(&*response).map_err(|e| Error::from(Error::InvalidResponse(format!("{:?}", e))))?;

    match response {
        rpc::Response::Single(output) => helpers::to_result_from_output(output),
        _ => Err(Error::InvalidResponse("Expected single, got batch.".into()).into()),
    }
}

/// Parse bytes RPC batch response into `Result`.
fn batch_response<T: Deref<Target = [u8]>>(response: T) -> Result<Vec<Result<rpc::Value>>> {
    let response =
        serde_json::from_slice(&*response).map_err(|e| Error::from(Error::InvalidResponse(format!("{:?}", e))))?;

    match response {
        rpc::Response::Batch(outputs) => Ok(outputs.into_iter().map(helpers::to_result_from_output).collect()),
        _ => Err(Error::InvalidResponse("Expected batch, got single.".into()).into()),
    }
}
