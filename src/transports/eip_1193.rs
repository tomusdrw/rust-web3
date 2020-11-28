//! EIP-1193 transport
//!
//! This transport lets you use the library inside a browser to interact with
//! EIP-1193 providers like MetaMask. It's intended for use with Rust's
//! WebAssembly target.

#![cfg(feature = "eip-1193")]

use crate::api::SubscriptionId;
use crate::{error, DuplexTransport, Error, RequestId, Transport};
use futures::channel::mpsc;
use futures::future;
use futures::future::LocalBoxFuture;
use jsonrpc_core::types::request::{Call, MethodCall};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;
use wasm_bindgen::prelude::*;

/// EIP-1193 transport
#[derive(Clone, Debug)]
pub struct Eip1193 {
    provider: Provider,
    subscriptions: Rc<RefCell<BTreeMap<SubscriptionId, mpsc::UnboundedSender<serde_json::Value>>>>,
}

impl Eip1193 {
    /// Build an EIP-1193 transport.
    pub fn new(provider: Provider) -> Self {
        let subscriptions: Rc<RefCell<BTreeMap<SubscriptionId, mpsc::UnboundedSender<serde_json::Value>>>> =
            Rc::new(RefCell::new(BTreeMap::new()));
        let subscriptions_for_closure = subscriptions.clone();
        let msg_handler = Closure::wrap(Box::new(move |evt_js: JsValue| {
            let evt: serde_json::Value = evt_js.into_serde().expect("couldn't translate notification via JSON");
            log::trace!("Message from provider: {}", evt);
            match evt.get("type").and_then(serde_json::Value::as_str) {
                Some("eth_subscription") => {
                    let data = evt
                        .get("data")
                        .and_then(serde_json::Value::as_object)
                        .expect("couldn't get data field");
                    let subscription = data
                        .get("subscription")
                        .and_then(serde_json::Value::as_str)
                        .expect("couldn't get subscription field");
                    let result = data.get("result").expect("couldn't get result field").clone();
                    let subscriptions_map = subscriptions_for_closure.borrow();
                    match subscriptions_map.get(&SubscriptionId::from(String::from(subscription))) {
                        Some(sink) => {
                            if let Err(err) = sink.unbounded_send(result) {
                                log::error!("Error sending notification: {}", err)
                            }
                        }
                        None => log::warn!("Got message for non-existent subscription {}", subscription),
                    }
                }
                Some(other) => log::warn!("Got unknown notification type: {}", other),
                None => log::error!("Got notification with no \"type\" field: {:?}", evt),
            }
        }) as Box<dyn FnMut(JsValue)>);
        provider.on("message", &msg_handler);
        msg_handler.into_js_value();
        Eip1193 {
            provider,
            subscriptions,
        }
    }
}

impl Transport for Eip1193 {
    type Out = LocalBoxFuture<'static, error::Result<serde_json::value::Value>>;

    fn prepare(&self, method: &str, params: Vec<serde_json::Value>) -> (RequestId, Call) {
        // EIP-1193 uses the JSON-RPC function API, but it isn't actually JSON-RPC, so some of
        // these fields are ignored.
        (
            0,
            Call::from(MethodCall {
                jsonrpc: None,
                method: String::from(method),
                params: jsonrpc_core::types::Params::Array(params),
                id: jsonrpc_core::types::Id::Null,
            }),
        )
    }

    fn send(&self, _id: RequestId, request: Call) -> Self::Out {
        match request {
            Call::MethodCall(method_call) => match method_call.params {
                jsonrpc_core::types::Params::Array(ref params) => {
                    let js_params = js_sys::Array::from(
                        &JsValue::from_serde(params).expect("couldn't send method params via JSON"),
                    );
                    let copy = self.provider.clone();
                    Box::pin(async move {
                        copy.request_wrapped(RequestArguments {
                            method: method_call.method,
                            params: js_params,
                        })
                        .await
                    })
                }
                _other => Box::pin(future::err(Error::Internal)),
            },
            _other => Box::pin(future::err(Error::Internal)),
        }
    }
}

impl DuplexTransport for Eip1193 {
    type NotificationStream = mpsc::UnboundedReceiver<serde_json::Value>;

    fn subscribe(&self, id: SubscriptionId) -> error::Result<Self::NotificationStream> {
        let (sender, receiver) = mpsc::unbounded();
        let mut subscriptions_ref = self.subscriptions.borrow_mut();
        subscriptions_ref.insert(id, sender);
        Ok(receiver)
    }

    fn unsubscribe(&self, id: SubscriptionId) -> error::Result<()> {
        match (*self.subscriptions.borrow_mut()).remove(&id) {
            Some(_sender) => Ok(()),
            None => Err(Error::Internal),
        }
    }
}

#[wasm_bindgen]
// Rustfmt removes the 'async' keyword from async functions in extern blocks. It's fixed
// in rustfmt 2.
#[rustfmt::skip]
extern "C" {
    #[wasm_bindgen]
    #[derive(Clone, Debug)]
    /// An EIP-1193 provider object. Available by convention at `window.ethereum`
    pub type Provider;

    #[wasm_bindgen(catch, method)]
    async fn request(_: &Provider, args: RequestArguments) -> Result<JsValue, JsValue>;

    #[wasm_bindgen(method)]
    fn on(_: &Provider, eventName: &str, listener: &Closure<dyn FnMut(JsValue)>);
}

impl Provider {
    /// Get the provider at `window.ethereum`.
    pub fn get_default() -> Result<Self, JsValue> {
        get_provider_js()
    }

    async fn request_wrapped(&self, args: RequestArguments) -> error::Result<serde_json::value::Value> {
        let js_result = self.request(args).await;
        match js_result {
            Ok(res) => Ok(res.into_serde().expect("couldn't translate request via JSON")),
            Err(err) => {
                match err.into_serde() {
                    Ok(json_rpc_err) => Err(Error::Rpc(json_rpc_err)),
                    // THE EIP says an error response MUST match the JSON RPC error structure.
                    Err(_) => Err(Error::InvalidResponse(format!("{:?}", err))),
                }
            }
        }
    }
}

#[wasm_bindgen(inline_js = "export function get_provider_js() {return window.ethereum}")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn get_provider_js() -> Result<Provider, JsValue>;
}

#[wasm_bindgen]
struct RequestArguments {
    method: String,
    params: js_sys::Array,
}

#[wasm_bindgen]
impl RequestArguments {
    #[wasm_bindgen(getter)]
    pub fn method(&self) -> String {
        self.method.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn params(&self) -> js_sys::Array {
        self.params.clone()
    }
}
