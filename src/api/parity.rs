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
    "blockHash": null,
    "blockNumber": null,
    "creates": null,
    "from": "0xee3ea02840129123d5397f91be0391283a25bc7d",
    "gas": "0x23b58",
    "gasPrice": "0xba43b7400",
    "hash": "0x160b3c30ab1cf5871083f97ee1cee3901cfba3b0a2258eb337dd20a7e816b36e",
    "input": "0x095ea7b3000000000000000000000000bf4ed7b27f1d666546e30d74d50d173d20bca75400000000000000000000000000002643c948210b4bd99244ccd64d5555555555",
    "condition": {
    "block": 1
    },
    "chainId": 1,
    "nonce": "0x5",
    "publicKey": "0x96157302dade55a1178581333e57d60ffe6fdf5a99607890456a578b4e6b60e335037d61ed58aa4180f9fd747dc50d44a7924aa026acbfb988b5062b629d6c36",
    "r": "0x92e8beb19af2bad0511d516a86e77fa73004c0811b2173657a55797bdf8558e1",
    "raw": "0xf8aa05850ba43b740083023b5894bb9bc244d798123fde783fcc1c72d3bb8c18941380b844095ea7b3000000000000000000000000bf4ed7b27f1d666546e30d74d50d173d20bca75400000000000000000000000000002643c948210b4bd99244ccd64d555555555526a092e8beb19af2bad0511d516a86e77fa73004c0811b2173657a55797bdf8558e1a062b4d4d125bbcb9c162453bc36ca156537543bb4414d59d1805d37fb63b351b8",
    "s": "0x62b4d4d125bbcb9c162453bc36ca156537543bb4414d59d1805d37fb63b351b8",
    "standardV": "0x1",
    "to": "0xbb9bc244d798123fde783fcc1c72d3bb8c189413",
    "transactionIndex": null,
    "v": "0x26",
    "value": "0x0"
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
                transaction_type: None,
                access_list: None,
            },
            CallRequest {
                from: Some(Address::from_low_u64_be(0x321)),
                to: Some(Address::from_low_u64_be(0x123)),
                gas: None,
                gas_price: None,
                value: None,
                data: Some(hex!("0493").into()),
                transaction_type: None,
                access_list: None,
            },
            CallRequest {
                from: None,
                to: Some(Address::from_low_u64_be(0x765)),
                gas: None,
                gas_price: None,
                value: Some(0x5.into()),
                data: Some(hex!("0723").into()),
                transaction_type: None,
                access_list: None,
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
