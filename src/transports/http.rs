//! HTTP Transport

extern crate hyper;

use std::ops::Deref;
use std::sync::atomic::{self, AtomicUsize};

use futures::sync::{mpsc, oneshot};
use futures::{self, future, Future, sink, Sink, Stream};
use helpers;
use parking_lot::Mutex;
use rpc;
use serde_json;
use transports::Result;
use transports::shared::{EventLoopHandle, Response};
use transports::tokio_core::reactor;
use {BatchTransport, Transport, Error, RequestId};

impl From<hyper::Error> for Error {
  fn from(err: hyper::Error) -> Self {
    Error::Transport(format!("{:?}", err))
  }
}

impl From<hyper::error::UriError> for Error {
  fn from(err: hyper::error::UriError) -> Self {
    Error::Transport(format!("{:?}", err))
  }
}

// The max string length of a request without transfer-encoding: chunked.
const MAX_SINGLE_CHUNK: usize = 256;
const DEFAULT_MAX_PARALLEL: usize = 64;
type Pending = oneshot::Sender<Result<hyper::Chunk>>;

/// A future representing pending HTTP request, resolves to a response.
pub type FetchTask<F> = Response<F, hyper::Chunk>;

/// HTTP Transport (synchronous)
pub struct Http {
  id: AtomicUsize,
  url: hyper::Uri,
  write_sender: Mutex<sink::Wait<mpsc::Sender<(
    hyper::client::Request,
    Pending
  )>>>,
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
    EventLoopHandle::spawn(move |handle| {
        Self::with_event_loop(&url, handle, max_parallel)
    })
  }

  /// Create new HTTP transport with given URL and existing event loop handle.
  pub fn with_event_loop(url: &str, handle: &reactor::Handle, max_parallel: usize) -> Result<Self> {
    let (write_sender, write_receiver) = mpsc::channel(1024);
    let client = hyper::Client::new(handle);

    handle.spawn(write_receiver
      .map(move |(request, tx): (_, Pending)| {
        client.request(request).then(move |response| {
          Ok((response, tx))
        })
      })
      .buffer_unordered(max_parallel)
      .for_each(|(response, tx)| {
        use futures::future::Either::{A, B};
        let future = match response {
          Ok(ref res) if !res.status().is_success() => {
            A(future::err(Error::Transport(format!("Unexpected response status code: {}", res.status()))))
          },
          Ok(res) => B(res.body().concat2().map_err(Into::into)),
          Err(err) => A(future::err(err.into())),
        };
        future.then(move |result| {
          if let Err(err) = tx.send(result) {
            warn!("Error resuming asynchronous request: {:?}", err);
          }
          Ok(())
        })
      })
    );

    Ok(Http {
      id: Default::default(),
      url: url.parse()?,
      write_sender: Mutex::new(write_sender.wait()),
    })
  }

  fn send_request<F, O>(&self, id: RequestId, request: rpc::Request, extract: F) -> FetchTask<F> where
    F: Fn(hyper::Chunk) -> O,
  {
    let request = helpers::to_string(&request);
    debug!("[{}] Sending: {} to {}", id, request, self.url);

    let mut req = hyper::client::Request::new(hyper::Method::Post, self.url.clone());
    req.headers_mut().set(hyper::header::ContentType::json());
    req.headers_mut().set(hyper::header::UserAgent::new("web3.rs"));
    let len = request.len();
    // Don't send chunked request
    if len < MAX_SINGLE_CHUNK {
      req.headers_mut().set(hyper::header::ContentLength(len as u64));
    }
    req.set_body(request);

    let (tx, rx) = futures::oneshot();
    let result = {
      let mut sender = self.write_sender.lock();
      (*sender).send((req, tx)).map_err(|err| Error::Transport(format!("{:?}", err)))
    };

    Response::new(id, result, rx, extract)
  }
}

impl Transport for Http {
  type Out = FetchTask<fn (hyper::Chunk) -> Result<rpc::Value>>;

  fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
    let id = self.id.fetch_add(1, atomic::Ordering::AcqRel);
    let request = helpers::build_request(id, method, params);

    (id, request)
  }

  fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
    self.send_request(
      id,
      rpc::Request::Single(request),
      single_response,
    )
  }
}

impl BatchTransport for Http {
  type Batch = FetchTask<fn (hyper::Chunk) -> Result<Vec<Result<rpc::Value>>>>;

  fn send_batch<T>(&self, requests: T) -> Self::Batch where
    T: IntoIterator<Item=(RequestId, rpc::Call)>
  {
    let mut it = requests.into_iter();
    let (id, first) = it.next().map(|x| (x.0, Some(x.1))).unwrap_or_else(|| (0, None));
    let requests = first.into_iter().chain(it.map(|x| x.1)).collect();

    self.send_request(
      id,
      rpc::Request::Batch(requests),
      batch_response,
    )
  }
}

/// Parse bytes RPC response into `Result`.
fn single_response<T: Deref<Target=[u8]>>(response: T) -> Result<rpc::Value> {
  let response = serde_json::from_slice(&*response)
    .map_err(|e| Error::InvalidResponse(format!("{:?}", e)))?;

  match response {
    rpc::Response::Single(output) => helpers::to_result_from_output(output),
    _ => Err(Error::InvalidResponse("Expected single, got batch.".into())),
  }
}

/// Parse bytes RPC batch response into `Result`.
fn batch_response<T: Deref<Target=[u8]>>(response: T) -> Result<Vec<Result<rpc::Value>>> {
  let response = serde_json::from_slice(&*response)
    .map_err(|e| Error::InvalidResponse(format!("{:?}", e)))?;

  match response {
    rpc::Response::Batch(outputs) => Ok(outputs.into_iter().map(helpers::to_result_from_output).collect()),
    _ => Err(Error::InvalidResponse("Expected batch, got single.".into())),
  }
}

