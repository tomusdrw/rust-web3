//! Test Transport

use crate::error::{self, Error};
use crate::helpers;
use crate::rpc;
use crate::{RequestId, Transport};
use futures::future;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::marker::Unpin;
use std::rc::Rc;

type Result<T> = Box<dyn futures::Future<Output = error::Result<T>> + Send + Unpin>;

/// Test Transport
#[derive(Debug, Default, Clone)]
pub struct TestTransport {
    asserted: usize,
    requests: Rc<RefCell<Vec<(String, Vec<rpc::Value>)>>>,
    responses: Rc<RefCell<VecDeque<rpc::Value>>>,
}

impl Transport for TestTransport {
    type Out = Result<rpc::Value>;

    fn prepare(&self, method: &str, params: Vec<rpc::Value>) -> (RequestId, rpc::Call) {
        let request = helpers::build_request(1, method, params.clone());
        self.requests.borrow_mut().push((method.into(), params));
        (self.requests.borrow().len(), request)
    }

    fn send(&self, id: RequestId, request: rpc::Call) -> Result<rpc::Value> {
        Box::new(future::ready(match self.responses.borrow_mut().pop_front() {
            Some(response) => Ok(response),
            None => {
                println!("Unexpected request (id: {:?}): {:?}", id, request);
                Err(Error::Unreachable)
            }
        }))
    }
}

impl TestTransport {
    pub fn set_response(&mut self, value: rpc::Value) {
        *self.responses.borrow_mut() = vec![value].into();
    }

    pub fn add_response(&mut self, value: rpc::Value) {
        self.responses.borrow_mut().push_back(value);
    }

    pub fn assert_request(&mut self, method: &str, params: &[String]) {
        let idx = self.asserted;
        self.asserted += 1;

        let (m, p) = self.requests.borrow().get(idx).expect("Expected result.").clone();
        assert_eq!(&m, method);
        let p: Vec<String> = p.into_iter().map(|p| serde_json::to_string(&p).unwrap()).collect();
        assert_eq!(p, params);
    }

    pub fn assert_no_more_requests(&self) {
        let requests = self.requests.borrow();
        assert_eq!(
            self.asserted,
            requests.len(),
            "Expected no more requests, got: {:?}",
            &requests[self.asserted..]
        );
    }
}
