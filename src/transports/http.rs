//! HTTP Transport

extern crate reqwest;

use std::io::{self, Read};
use std::sync::Arc;
use std::sync::atomic::{self, AtomicUsize};

use futures::{self, Future};
use helpers;
use rpc;
use {Transport, Error as RpcError};

impl From<reqwest::Error> for RpcError {
  fn from(err: reqwest::Error) -> Self {
    RpcError::Transport(format!("{:?}", err))
  }
}

impl From<io::Error> for RpcError {
  fn from(err: io::Error) -> Self {
    RpcError::Transport(format!("{:?}", err))
  }
}

/// HTTP Transport (synchronous)
pub struct Http {
  id: AtomicUsize,
  client: Arc<reqwest::Client>,
  url: String,
}

impl Http {
  /// Create new HTTP transport with given URL
  pub fn new(url: &str) -> Result<Self, reqwest::Error> {
    let mut client = reqwest::Client::new()?;
		client.redirect(reqwest::RedirectPolicy::limited(1));

    Ok(Http {
      id: Default::default(),
      client: Arc::new(client),
      url: url.into(),
    })
  }
}

impl Transport for Http {
  type Out = FetchTask;

  fn execute(&self, method: &str, params: Vec<rpc::Value>) -> FetchTask {
    let id = self.id.fetch_add(1, atomic::Ordering::Relaxed);
    let request = helpers::build_request(id, method, params);
    debug!("Calling: {}", request);

    FetchTask {
      id: id,
      url: self.url.clone(),
      client: self.client.clone(),
      request: request,
    }
  }
}

/// Future which will represents a task to fetch data.
/// Will execute synchronously when first polled.
pub struct FetchTask {
  id: usize,
	url: String,
	client: Arc<reqwest::Client>,
  request: String,
}

impl Future for FetchTask {
	type Item = rpc::Value;
	type Error = RpcError;

	fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
		trace!("[{}] Starting fetch task.", self.id);
		let mut result = self.client.post(&self.url)
              .body(self.request.as_str())
						  .header(reqwest::header::ContentType::json())
						  .header(reqwest::header::UserAgent("web3.rs".into()))
						  .send()?;

    trace!("[{}] Finished fetch.", self.id);

    let mut response = String::new();
    result.read_to_string(&mut response)?;
    trace!("[{}] Response read: {}", self.id, response);

    let response = helpers::to_result(&response)?;

    debug!("[{}] Success: {}", self.id, response);

		Ok(futures::Async::Ready(response))
	}
}

