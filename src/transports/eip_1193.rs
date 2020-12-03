//! EIP-1193 transport
//!
//! This transport lets you use the library inside a browser to interact with
//! EIP-1193 providers like MetaMask. It's intended for use with Rust's
//! WebAssembly target.

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

type Subscriptions = Rc<RefCell<BTreeMap<SubscriptionId, mpsc::UnboundedSender<serde_json::Value>>>>;

/// EIP-1193 transport
#[derive(Clone, Debug)]
pub struct Eip1193 {
    provider: Provider,
    subscriptions: Subscriptions,
}

impl Eip1193 {
    /// Build an EIP-1193 transport.
    pub fn new(provider: Provider) -> Self {
        let subscriptions: Subscriptions = Subscriptions::default();
        let subscriptions_for_closure = subscriptions.clone();
        let msg_handler = Closure::wrap(Box::new(move |evt_js: JsValue| {
            let evt = evt_js.into_serde::<Event>().expect("Couldn't parse event data");
            log::trace!("Message from provider: {:?}", evt);
            match evt.event_type.as_str() {
                "eth_subscription" => {
                    let subscriptions_map = subscriptions_for_closure.borrow();
                    match subscriptions_map.get(&SubscriptionId::from(evt.data.subscription.clone())) {
                        Some(sink) => {
                            if let Err(err) = sink.unbounded_send(evt.data.result) {
                                log::error!("Error sending notification: {}", err)
                            }
                        }
                        None => log::warn!("Got message for non-existent subscription {}", evt.data.subscription),
                    }
                }
                other => log::warn!("Got unknown notification type: {}", other),
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

/// Event data sent from the JavaScript side to our callback.
#[derive(serde::Deserialize, Debug)]
struct Event {
    #[serde(rename = "type")]
    event_type: String,
    data: EventData,
}

#[derive(serde::Deserialize, Debug)]
struct EventData {
    subscription: String,
    result: serde_json::Value,
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
            Call::MethodCall(MethodCall {
                params: jsonrpc_core::types::Params::Array(params),
                method,
                ..
            }) => {
                let js_params =
                    js_sys::Array::from(&JsValue::from_serde(&params).expect("couldn't send method params via JSON"));
                let copy = self.provider.clone();
                Box::pin(async move {
                    copy.request_wrapped(RequestArguments {
                        method,
                        params: js_params,
                    })
                    .await
                })
            }
            _ => panic!("Can't send JSON-RPC requests other than method calls with EIP-1193 transport!"),
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
            None => panic!("Tried to unsubscribe from non-existent subscription. Did we already unsubscribe?"),
        }
    }
}

#[wasm_bindgen]
// Rustfmt removes the 'async' keyword from async functions in extern blocks. It's fixed
// in rustfmt 2.
#[rustfmt::skip]
extern "C" {
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
    pub fn default() -> Result<Self, JsValue> {
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
