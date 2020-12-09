use crate::{
    api::Namespace,
    helpers::{self, CallFuture},
    types::{Bytes, CallRequest},
    Transport,
};

/// `Parity` namespace
#[derive(Debug, Clone)]
pub struct Parity<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for Parity<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        Parity { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Parity<T> {
    /// Sequentially call multiple contract methods in one request without changing the state of the blockchain.
    pub fn call(&self, reqs: Vec<CallRequest>) -> CallFuture<Vec<Bytes>, T::Out> {
        let reqs = helpers::serialize(&reqs);

        CallFuture::new(self.transport.execute("parity_call", vec![reqs]))
    }
}

#[cfg(test)]
mod tests {
    use super::Parity;
    use crate::{
        api::Namespace,
        rpc::Value,
        types::{Address, CallRequest},
    };
    use hex_literal::hex;

    rpc_test!(
        Parity:call,
        vec![
            CallRequest {
                from: None,
                to: Some(Address::from_low_u64_be(0x123)),
                gas: None,
                gas_price: None,
                value: Some(0x1.into()),
                data: None,
            },
            CallRequest {
                from: Some(Address::from_low_u64_be(0x321)),
                to: Some(Address::from_low_u64_be(0x123)),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(hex!("0493").into()),
            },
            CallRequest {
                from: None,
                to: Some(Address::from_low_u64_be(0x765)),
                gas: None,
                gas_price: None,
                value: Some(0x5.into()),
                data: Some(hex!("0723").into())
            }
        ] => "parity_call", vec![
            r#"[{"to":"0x0000000000000000000000000000000000000123","value":"0x1"},{"data":"0x0493","from":"0x0000000000000000000000000000000000000321","to":"0x0000000000000000000000000000000000000123"},{"data":"0x0723","to":"0x0000000000000000000000000000000000000765","value":"0x5"}]"#
        ];
        Value::Array(vec![Value::String("0x010203".into()), Value::String("0x7198ab".into()), Value::String("0xde763f".into())]) => vec![hex!("010203").into(), hex!("7198ab").into(), hex!("de763f").into()]
    );
}
