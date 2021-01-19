use crate::{
    api::Namespace,
    helpers::{self, CallFuture},
    rpc::Value,
    types::{Bytes, CallRequest, ParityPendingTransactionFilter, Transaction},
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

    /// Get pending transactions
    /// Blocked by https://github.com/openethereum/openethereum/issues/159
    pub fn pending_transactions(
        &self,
        limit: Option<usize>,
        filter: Option<ParityPendingTransactionFilter>,
    ) -> CallFuture<Vec<Transaction>, T::Out> {
        let limit = limit.map(Value::from);
        let filter = filter.as_ref().map(helpers::serialize);
        let params = match (limit, filter) {
            (l, Some(f)) => vec![l.unwrap_or(Value::Null), f],
            (Some(l), None) => vec![l],
            _ => vec![],
        };

        CallFuture::new(self.transport.execute("parity_pendingTransactions", params))
    }
}

#[cfg(test)]
mod tests {
    use super::Parity;
    use crate::{
        api::Namespace,
        rpc::Value,
        types::{Address, CallRequest, FilterCondition, ParityPendingTransactionFilter, Transaction, U64},
    };
    use hex_literal::hex;

    const EXAMPLE_PENDING_TX: &str = r#"{
    "hash": "0xc6ef2fc5426d6ad6fd9e2a26abeab0aa2411b7ab17f30a99d3cb96aed1d1055b",
    "nonce": "0x0",
    "blockHash": null,
    "blockNumber": null,
    "transactionIndex": null,
    "from": "0x407d73d8a49eeb85d32cf465507dd71d507100c1",
    "to":   "0x85dd43d8a49eeb85d32cf465507dd71d507100c1",
    "value": "0x7f110",
    "gas": "0x7f110",
    "gasPrice": "0x09184e72a000",
    "input": "0x603880600c6000396000f300603880600c6000396000f3603880600c6000396000f360"
  }"#;

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

    rpc_test!(
        Parity:pending_transactions,
        1,
        ParityPendingTransactionFilter::builder()
            .from(Address::from_low_u64_be(0x32))
            .gas(U64::from(100_000))
            .gas_price(FilterCondition::GreaterThan(U64::from(100_000_000_000 as u64)))
            .build()
         => "parity_pendingTransactions",
            vec![r#"1"#, r#"{"from":{"eq":"0x0000000000000000000000000000000000000032"},"gas":{"eq":"0x186a0"},"gas_price":{"gt":"0x174876e800"}}"#]
        ;
        Value::Array(vec![::serde_json::from_str(EXAMPLE_PENDING_TX).unwrap()])
      => vec![::serde_json::from_str::<Transaction>(EXAMPLE_PENDING_TX).unwrap()]
    );
}
