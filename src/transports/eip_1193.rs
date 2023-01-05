//! EIP-1193 transport
//!
//! This transport lets you use the library inside a browser to interact with
//! EIP-1193 providers like MetaMask. It's intended for use with Rust's
//! WebAssembly target.

use crate::{
    api::SubscriptionId,
    error,
    types::{Address, U256},
    DuplexTransport, Error, RequestId, Transport,
};
use futures::{channel::mpsc, future::LocalBoxFuture, Stream};
use jsonrpc_core::{
    error::{Error as RPCError, ErrorCode as RPCErrorCode},
    types::request::{Call, MethodCall},
};
use serde::{
    de::{value::StringDeserializer, DeserializeOwned, IntoDeserializer},
    Deserialize, Serialize,
};
use serde_wasm_bindgen::Serializer;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
use wasm_bindgen::{prelude::*, JsCast};

type Subscriptions = Rc<RefCell<BTreeMap<SubscriptionId, mpsc::UnboundedSender<serde_json::Value>>>>;

/// EIP-1193 transport
#[derive(Clone, Debug)]
pub struct Eip1193 {
    provider_and_listeners: Rc<RefCell<ProviderAndListeners>>,
    subscriptions: Subscriptions,
}

impl Eip1193 {
    /// Build an EIP-1193 transport.
    pub fn new(provider: Provider) -> Self {
        let mut provider_and_listeners = ProviderAndListeners {
            provider,
            listeners: BTreeMap::new(),
        };
        let subscriptions: Subscriptions = Subscriptions::default();
        let subscriptions_for_closure = subscriptions.clone();
        let msg_handler = Closure::wrap(Box::new(move |evt_js: JsValue| {
            let evt = deserialize_from_js::<MessageEvent>(evt_js).expect("Couldn't parse event data");
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
        provider_and_listeners.on("message", msg_handler);
        Eip1193 {
            provider_and_listeners: Rc::new(RefCell::new(provider_and_listeners)),
            subscriptions,
        }
    }

    /// Get a `Stream` of `connect` events, and the `chainId`s connected to. NB the delivery on these
    /// is unreliable, sometimes you get one on page load and sometimes you don't. The `Stream` will
    /// be closed when the `Eip1193` is dropped.
    pub fn connect_stream(&self) -> impl Stream<Item = Option<String>> {
        self.handle_ad_hoc_event("connect", |evt_js| {
            let evt = deserialize_from_js::<ConnectEvent>(evt_js).expect("couldn't parse connect event");
            evt.chain_id
        })
    }

    /// Get a `Stream` of `disconnect` events, with the associated errors. Same drop behavior as
    /// above.
    pub fn disconnect_stream(&self) -> impl Stream<Item = jsonrpc_core::Error> {
        self.handle_ad_hoc_event("disconnect", |evt_js| {
            deserialize_from_js(evt_js).expect("deserializing disconnect error failed")
        })
    }

    /// Get a `Stream` of `chainChanged` events, with the `chainId`s changed to. Same drop behavior
    /// as above.
    pub fn chain_changed_stream(&self) -> impl Stream<Item = U256> {
        self.handle_ad_hoc_event("chainChanged", |evt_js| {
            Self::deserialize_js_string(&evt_js, "chain changed event")
        })
    }

    /// Get a `Stream` of `accountsChanged` events, with the addresses that are now accessible. Same
    /// drop behavior as above.
    pub fn accounts_changed_stream(&self) -> impl Stream<Item = Vec<Address>> {
        self.handle_ad_hoc_event("accountsChanged", |evt_js| {
            js_sys::Array::unchecked_from_js(evt_js)
                .iter()
                .map(|js_str| Self::deserialize_js_string(&js_str, "account address"))
                .collect()
        })
    }

    /// Helper function for handling events other than `message`. Given the event name and a
    /// processor function, create a stream of processed events.
    fn handle_ad_hoc_event<T, F>(&self, name: &str, handler: F) -> impl Stream<Item = T>
    where
        F: Fn(JsValue) -> T + 'static,
        T: 'static,
    {
        let (sender, receiver) = mpsc::unbounded();
        self.provider_and_listeners.borrow_mut().on(
            name,
            Closure::wrap(Box::new(move |evt| {
                let evt_parsed = handler(evt);
                if let Err(err) = sender.unbounded_send(evt_parsed) {
                    log::error!("Couldn't send ad hoc event to channel: {}", err)
                }
            })),
        );
        receiver
    }

    /// Helper function for serde-deserializing from `JsString`s. Panics on failure.
    fn deserialize_js_string<'de, O: Deserialize<'de>>(js_str: &JsValue, name: &str) -> O {
        let deserializer: StringDeserializer<serde::de::value::Error> = js_str
            .as_string()
            .expect(&format!("{:?} not a string", js_str))
            .into_deserializer();
        O::deserialize(deserializer).expect(&format!("couldn't deserialize {}", name))
    }
}

/// Event data sent from the JavaScript side to our callback.
#[derive(serde::Deserialize, Debug)]
struct MessageEvent {
    #[serde(rename = "type")]
    event_type: String,
    data: MessageEventData,
}

#[derive(serde::Deserialize, Debug)]
struct MessageEventData {
    subscription: String,
    result: serde_json::Value,
}

#[derive(serde::Deserialize)]
struct ConnectEvent {
    #[serde(rename = "chainId")]
    chain_id: Option<String>,
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
                    js_sys::Array::from(&serialize_to_js(&params).expect("couldn't send method params via JSON"));
                let copy = self.provider_and_listeners.borrow().provider.clone();
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

    #[wasm_bindgen(method, js_name = "removeListener")]
    fn removeListener(_: &Provider, eventName: &str, listener: &Closure<dyn FnMut(JsValue)>);
}

impl Provider {
    /// Get the provider at `window.ethereum`.
    pub fn default() -> Result<Option<Self>, JsValue> {
        get_provider_js()
    }

    fn parse_response(resp: Result<JsValue, JsValue>) -> error::Result<serde_json::value::Value> {
        // Fix #544
        #[derive(Debug, Deserialize)]
        pub struct RPCErrorExtra {
            /// Code
            pub code: RPCErrorCode,
            /// Message
            pub message: String,
            /// Optional data
            pub data: Option<serde_json::value::Value>,
            /// Optional stack
            pub stack: Option<serde_json::value::Value>,
        }

        impl Into<RPCError> for RPCErrorExtra {
            fn into(self) -> RPCError {
                RPCError {
                    code: self.code,
                    message: self.message,
                    data: self.data,
                }
            }
        }

        let parsed_value = resp
            .map(deserialize_from_js)
            .map_err(deserialize_from_js::<RPCErrorExtra>);
        match parsed_value {
            Ok(Ok(res)) => Ok(res),
            Err(Ok(err)) => Err(Error::Rpc(err.into())),
            err => Err(Error::InvalidResponse(format!("{:?}", err))),
        }
    }

    async fn request_wrapped(&self, args: RequestArguments) -> error::Result<serde_json::value::Value> {
        let response = self.request(args).await;
        Self::parse_response(response)
    }
}

/// Keep the provider and the event listeners attached to it together so we can remove them in the
/// `Drop` implementation. The logic can't go in Eip1193 because it's `Clone`, and cloning a JS
/// object just clones the reference.
#[derive(Debug)]
struct ProviderAndListeners {
    provider: Provider,
    listeners: BTreeMap<String, Vec<Closure<dyn FnMut(JsValue)>>>,
}

impl ProviderAndListeners {
    /// Listen for an event, and keep the listener closure for later cleanup.
    fn on(&mut self, name: &str, listener: Closure<dyn FnMut(JsValue)>) {
        self.provider.on(name, &listener);
        self.listeners
            .entry(name.to_owned())
            .or_insert(Vec::with_capacity(1))
            .push(listener);
    }
}

impl Drop for ProviderAndListeners {
    fn drop(&mut self) {
        for (event_name, listeners) in self.listeners.iter() {
            for listener in listeners.iter() {
                self.provider.removeListener(event_name, listener)
            }
        }
    }
}

#[wasm_bindgen(inline_js = "export function get_provider_js() {return window.ethereum}")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn get_provider_js() -> Result<Option<Provider>, JsValue>;
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

fn serialize_to_js<T: Serialize>(value: &T) -> Result<JsValue, serde_wasm_bindgen::Error> {
    value.serialize(&Serializer::json_compatible())
}

fn deserialize_from_js<T: DeserializeOwned>(value: JsValue) -> Result<T, serde_wasm_bindgen::Error> {
    serde_wasm_bindgen::from_value(value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;

    fn json_to_js(json: &str) -> JsValue {
        let json_value = serde_json::from_str::<serde_json::Value>(json).unwrap();
        let js_value = serialize_to_js(&json_value).unwrap();
        js_value
    }

    #[wasm_bindgen_test]
    fn parses_valid_response_correctly() {
        let value = serde_json::from_str::<serde_json::Value>(r#"[1, false, null, "string"]"#).unwrap();
        let response = Ok(serialize_to_js(&value).unwrap());
        let expected_result = Ok(value);
        assert_eq!(Provider::parse_response(response), expected_result);

        let response = Err(json_to_js(
            r#"{"code": 15, "message": "string1", "data": "string2", "stack": "string3"}"#,
        ));
        let expected_result = Err(Error::Rpc(RPCError {
            code: RPCErrorCode::from(15),
            message: "string1".to_string(),
            data: Some(serde_json::Value::String("string2".to_string())),
        }));
        assert_eq!(Provider::parse_response(response), expected_result);
    }

    #[wasm_bindgen_test]
    fn returns_error_on_invalid_response() {
        assert!(matches!(
            Provider::parse_response(Err(json_to_js(r#"{"code": "red", "message": ""}"#))),
            Err(Error::InvalidResponse(_))
        ));
        assert!(matches!(
            Provider::parse_response(Err(json_to_js(r#"{}"#))),
            Err(Error::InvalidResponse(_))
        ));
    }
}
