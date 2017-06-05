//! HTTP Transport

extern crate hyper;

use std::fmt;
use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{self, AtomicUsize};

use futures::{self, Future};
use helpers;
use rpc;
use transports::Result;
use {BatchTransport, Transport, Error as RpcError, RequestId};

impl From<hyper::Error> for RpcError {
  fn from(err: hyper::Error) -> Self {
    RpcError::Transport(format!("{:?}", err))
  }
}

impl From<io::Error> for RpcError {
  fn from(err: io::Error) -> Self {
    RpcError::Transport(format!("{:?}", err))
  }
}
//
/// HTTP Transport (synchronous)
pub struct Http {
  id: AtomicUsize,
  client: Arc<hyper::Client>,
  url: String,
}

impl Http {
  /// Create new HTTP transport with given URL
  pub fn new(url: &str) -> ::std::result::Result<Self, hyper::Error> {
    let mut client = hyper::Client::with_pool_config(hyper::client::pool::Config {
      max_idle: 1024,
    });
    client.set_redirect_policy(hyper::client::RedirectPolicy::FollowAll);

    Ok(Http {
      id: Default::default(),
      client: Arc::new(client),
      url: url.into(),
    })
  }
}

impl Transport for Http {
  type Out = FetchTask<fn (&str) -> Result<rpc::Value>>;

  fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
    let id = self.id.fetch_add(1, atomic::Ordering::Relaxed);
    let request = helpers::build_request(id, method, params);
    (id, request)
  }

  fn send(&self, id: RequestId, request: rpc::Call) -> Self::Out {
    let request = helpers::to_string(&rpc::Request::Single(request));
    debug!("Calling: {}", request);

    FetchTask {
      id: format!("{}", id),
      url: self.url.clone(),
      client: self.client.clone(),
      request,
      extract: helpers::to_result as fn(&str) -> Result<rpc::Value>,
    }
  }
}

impl BatchTransport for Http {
  type Batch = FetchTask<fn(&str) -> Result<Vec<Result<rpc::Value>>>>;

  fn send_batch(&self, requests: Vec<(RequestId, rpc::Call)>) -> Self::Batch {
    let id = requests.get(0).map(|x| x.0).unwrap_or(0);
    let requests = requests.into_iter().map(|x| x.1).collect();
    let request = helpers::to_string(&rpc::Request::Batch(requests));
    debug!("Calling: {}", request);

    FetchTask {
      id: format!("batch-{}", id),
      url: self.url.clone(),
      client: self.client.clone(),
      request,
      extract: helpers::to_batch_result as fn(&str) -> Result<Vec<Result<rpc::Value>>>,
    }
  }
}

/// Future which will represents a task to fetch data.
/// Will execute synchronously when first polled.
pub struct FetchTask<T> {
  id: String,
  url: String,
  client: Arc<hyper::Client>,
  request: String,
  extract: T,
}

impl<T, Out> Future for FetchTask<T> where
  T: Fn(&str) -> Result<Out>,
  Out: fmt::Debug,
{
  type Item = Out;
  type Error = RpcError;

  fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
    trace!("[{}] Starting fetch task.", self.id);
    let mut result = self.client.post(&self.url)
      .body(self.request.as_str())
      .header(hyper::header::ContentType::json())
      .header(hyper::header::UserAgent("web3.rs".into()))
      .send()?;

    trace!("[{}] Finished fetch.", self.id);

    let mut response = String::new();
    result.read_to_string(&mut response)?;
    trace!("[{}] Response read: {}", self.id, response);

    let response = (self.extract)(&response)?;

    debug!("[{}] Success: {:?}", self.id, response);

    Ok(futures::Async::Ready(response))
  }
}

