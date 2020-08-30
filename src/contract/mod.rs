//! Ethereum Contract Interface

use crate::api::{Accounts, Eth, Namespace};
use crate::confirm;
use crate::contract::tokens::{Detokenize, Tokenize};
use crate::signing;
use crate::types::{
    Address, BlockId, Bytes, CallRequest, FilterBuilder, TransactionCondition, TransactionParameters,
    TransactionReceipt, TransactionRequest, H256, U256,
};
use crate::Transport;
use futures::{
    future::{self, Either},
    Future, FutureExt, TryFutureExt,
};
use std::{collections::HashMap, hash::Hash, time};

pub mod deploy;
mod error;
mod result;
pub mod tokens;

pub use crate::contract::error::Error;
pub use crate::contract::result::{CallFuture, QueryResult};

/// Contract `Result` type.
pub type Result<T> = std::result::Result<T, Error>;

/// Contract Call/Query Options
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Options {
    /// Fixed gas limit
    pub gas: Option<U256>,
    /// Fixed gas price
    pub gas_price: Option<U256>,
    /// Value to transfer
    pub value: Option<U256>,
    /// Fixed transaction nonce
    pub nonce: Option<U256>,
    /// A condition to satisfy before including transaction.
    pub condition: Option<TransactionCondition>,
}

impl Options {
    /// Create new default `Options` object with some modifications.
    pub fn with<F>(func: F) -> Options
    where
        F: FnOnce(&mut Options),
    {
        let mut options = Options::default();
        func(&mut options);
        options
    }
}

/// Ethereum Contract Interface
#[derive(Debug, Clone)]
pub struct Contract<T: Transport> {
    address: Address,
    eth: Eth<T>,
    abi: ethabi::Contract,
}

impl<T: Transport> Contract<T> {
    /// Creates deployment builder for a contract given it's ABI in JSON.
    pub fn deploy(eth: Eth<T>, json: &[u8]) -> ethabi::Result<deploy::Builder<T>> {
        let abi = ethabi::Contract::load(json)?;
        Ok(deploy::Builder {
            eth,
            abi,
            options: Options::default(),
            confirmations: 1,
            poll_interval: time::Duration::from_secs(7),
            linker: HashMap::default(),
        })
    }

    /// test
    pub fn deploy_from_truffle<S>(
        eth: Eth<T>,
        json: &[u8],
        linker: HashMap<S, Address>,
    ) -> ethabi::Result<deploy::Builder<T>>
    where
        S: AsRef<str> + Eq + Hash,
    {
        let abi = ethabi::Contract::load(json)?;
        let linker: HashMap<String, Address> = linker.into_iter().map(|(s, a)| (s.as_ref().to_string(), a)).collect();
        Ok(deploy::Builder {
            eth,
            abi,
            options: Options::default(),
            confirmations: 1,
            poll_interval: time::Duration::from_secs(7),
            linker,
        })
    }
}

impl<T: Transport> Contract<T> {
    /// Creates new Contract Interface given blockchain address and ABI
    pub fn new(eth: Eth<T>, address: Address, abi: ethabi::Contract) -> Self {
        Contract { address, eth, abi }
    }

    /// Creates new Contract Interface given blockchain address and JSON containing ABI
    pub fn from_json(eth: Eth<T>, address: Address, json: &[u8]) -> ethabi::Result<Self> {
        let abi = ethabi::Contract::load(json)?;
        Ok(Self::new(eth, address, abi))
    }

    /// Get the underlying contract ABI.
    pub fn abi(&self) -> &ethabi::Contract {
        &self.abi
    }

    /// Returns contract address
    pub fn address(&self) -> Address {
        self.address
    }

    /// Execute a contract function
    pub fn call<P>(&self, func: &str, params: P, from: Address, options: Options) -> CallFuture<H256, T::Out>
    where
        P: Tokenize,
    {
        self.abi
            .function(func)
            .and_then(|function| function.encode_input(&params.into_tokens()))
            .map(move |data| {
                let Options {
                    gas,
                    gas_price,
                    value,
                    nonce,
                    condition,
                } = options;

                self.eth
                    .send_transaction(TransactionRequest {
                        from,
                        to: Some(self.address),
                        gas,
                        gas_price,
                        value,
                        nonce,
                        data: Some(Bytes(data)),
                        condition,
                    })
                    .into()
            })
            .unwrap_or_else(Into::into)
    }

    /// Execute a signed contract function and wait for confirmations
    pub fn signed_call_with_confirmations<'a>(
        &'a self,
        func: &'a str,
        params: impl Tokenize,
        options: Options,
        confirmations: usize,
        key: impl signing::Key + 'a,
    ) -> impl Future<Output = crate::Result<TransactionReceipt>> + 'a {
        let poll_interval = time::Duration::from_secs(1);

        self.abi
            .function(func)
            .and_then(|function| function.encode_input(&params.into_tokens()))
            .map(move |fn_data| {
                let accounts = Accounts::new(self.eth.transport().clone());
                let mut tx = TransactionParameters {
                    nonce: options.nonce,
                    to: Some(self.address),
                    gas_price: options.gas_price,
                    data: Bytes(fn_data),
                    ..Default::default()
                };
                if let Some(gas) = options.gas {
                    tx.gas = gas;
                }
                if let Some(value) = options.value {
                    tx.value = value;
                }
                let sign_future = accounts.sign_transaction(tx, key);

                Either::Left(sign_future.and_then(move |signed| {
                    confirm::send_raw_transaction_with_confirmation(
                        self.eth.transport().clone(),
                        signed.raw_transaction,
                        poll_interval,
                        confirmations,
                    )
                }))
            })
            .unwrap_or_else(|e| {
                // TODO [ToDr] SendTransactionWithConfirmation should support custom error type (so that we can return
                // `contract::Error` instead of more generic `Error`.
                let err = crate::error::Error::Decoder(format!("{:?}", e));
                Either::Right(future::ready(Err(err)))
            })
    }

    /// Execute a contract function and wait for confirmations
    pub fn call_with_confirmations(
        &self,
        func: &str,
        params: impl Tokenize,
        from: Address,
        options: Options,
        confirmations: usize,
    ) -> confirm::SendTransactionWithConfirmation<T> {
        let poll_interval = time::Duration::from_secs(1);

        self.abi
            .function(func)
            .and_then(|function| function.encode_input(&params.into_tokens()))
            .map(|fn_data| {
                let transaction_request = TransactionRequest {
                    from,
                    to: Some(self.address),
                    gas: options.gas,
                    gas_price: options.gas_price,
                    value: options.value,
                    nonce: options.nonce,
                    data: Some(Bytes(fn_data)),
                    condition: options.condition,
                };

                confirm::send_transaction_with_confirmation(
                    self.eth.transport().clone(),
                    transaction_request,
                    poll_interval,
                    confirmations,
                )
            })
            .unwrap_or_else(|e| {
                // TODO [ToDr] SendTransactionWithConfirmation should support custom error type (so that we can return
                // `contract::Error` instead of more generic `Error`.
                confirm::SendTransactionWithConfirmation::from_err(
                    self.eth.transport().clone(),
                    crate::error::Error::Decoder(format!("{:?}", e)),
                )
            })
    }

    /// Estimate gas required for this function call.
    pub fn estimate_gas<P>(&self, func: &str, params: P, from: Address, options: Options) -> CallFuture<U256, T::Out>
    where
        P: Tokenize,
    {
        self.abi
            .function(func)
            .and_then(|function| function.encode_input(&params.into_tokens()))
            .map(|data| {
                self.eth
                    .estimate_gas(
                        CallRequest {
                            from: Some(from),
                            to: Some(self.address),
                            gas: options.gas,
                            gas_price: options.gas_price,
                            value: options.value,
                            data: Some(Bytes(data)),
                        },
                        None,
                    )
                    .into()
            })
            .unwrap_or_else(Into::into)
    }

    /// Call constant function
    pub fn query<R, A, B, P>(
        &self,
        func: &str,
        params: P,
        from: A,
        options: Options,
        block: B,
    ) -> QueryResult<R, T::Out>
    where
        R: Detokenize,
        A: Into<Option<Address>>,
        B: Into<Option<BlockId>>,
        P: Tokenize,
    {
        self.abi
            .function(func)
            .and_then(|function| {
                function
                    .encode_input(&params.into_tokens())
                    .map(|call| (call, function))
            })
            .map(|(call, function)| {
                let result = self.eth.call(
                    CallRequest {
                        from: from.into(),
                        to: Some(self.address),
                        gas: options.gas,
                        gas_price: options.gas_price,
                        value: options.value,
                        data: Some(Bytes(call)),
                    },
                    block.into(),
                );
                QueryResult::new(result, function.clone())
            })
            .unwrap_or_else(Into::into)
    }

    /// Find events matching the topics.
    pub fn events<A, B, C, R>(
        &self,
        event: &str,
        topic0: A,
        topic1: B,
        topic2: C,
    ) -> impl Future<Output = Result<Vec<R>>>
    where
        A: Tokenize,
        B: Tokenize,
        C: Tokenize,
        R: Detokenize,
    {
        fn to_topic<A: Tokenize>(x: A) -> ethabi::Topic<ethabi::Token> {
            let tokens = x.into_tokens();
            if tokens.is_empty() {
                ethabi::Topic::Any
            } else {
                tokens.into()
            }
        }

        let res = self.abi.event(event).and_then(|ev| {
            let filter = ev.filter(ethabi::RawTopicFilter {
                topic0: to_topic(topic0),
                topic1: to_topic(topic1),
                topic2: to_topic(topic2),
            })?;
            Ok((ev.clone(), filter))
        });
        let (ev, filter) = match res {
            Ok(x) => x,
            Err(e) => return Either::Left(future::ready(Err(e.into()))),
        };

        Either::Right(
            self.eth
                .logs(FilterBuilder::default().topic_filter(filter).build())
                .map_err(Into::into)
                .map(move |logs| {
                    logs.and_then(|logs| {
                        logs.into_iter()
                            .map(move |l| {
                                let log = ev.parse_log(ethabi::RawLog {
                                    topics: l.topics,
                                    data: l.data.0,
                                })?;

                                Ok(R::from_tokens(
                                    log.params.into_iter().map(|x| x.value).collect::<Vec<_>>(),
                                )?)
                            })
                            .collect::<Result<Vec<R>>>()
                    })
                }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Contract, Options};
    use crate::api::{self, Namespace};
    use crate::helpers::tests::TestTransport;
    use crate::rpc;
    use crate::types::{Address, BlockId, BlockNumber, H256, U256};
    use crate::Transport;

    fn contract<T: Transport>(transport: &T) -> Contract<&T> {
        let eth = api::Eth::new(transport);
        Contract::from_json(eth, Address::from_low_u64_be(1), include_bytes!("./res/token.json")).unwrap()
    }

    #[test]
    fn should_call_constant_function() {
        // given
        let mut transport = TestTransport::default();
        transport.set_response(rpc::Value::String("0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000c48656c6c6f20576f726c64210000000000000000000000000000000000000000".into()));

        let result: String = {
            let token = contract(&transport);

            // when
            futures::executor::block_on(token.query(
                "name",
                (),
                None,
                Options::default(),
                BlockId::Number(BlockNumber::Number(1.into())),
            ))
            .unwrap()
        };

        // then
        transport.assert_request(
            "eth_call",
            &[
                "{\"data\":\"0x06fdde03\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(),
                "\"0x1\"".into(),
            ],
        );
        transport.assert_no_more_requests();
        assert_eq!(result, "Hello World!".to_owned());
    }

    #[test]
    fn should_call_constant_function_by_hash() {
        // given
        let mut transport = TestTransport::default();
        transport.set_response(rpc::Value::String("0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000c48656c6c6f20576f726c64210000000000000000000000000000000000000000".into()));

        let result: String = {
            let token = contract(&transport);

            // when
            futures::executor::block_on(token.query(
                "name",
                (),
                None,
                Options::default(),
                BlockId::Hash(H256::default()),
            ))
            .unwrap()
        };

        // then
        transport.assert_request(
            "eth_call",
            &[
                "{\"data\":\"0x06fdde03\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(),
                "{\"blockHash\":\"0x0000000000000000000000000000000000000000000000000000000000000000\"}".into(),
            ],
        );
        transport.assert_no_more_requests();
        assert_eq!(result, "Hello World!".to_owned());
    }

    #[test]
    fn should_query_with_params() {
        // given
        let mut transport = TestTransport::default();
        transport.set_response(rpc::Value::String("0x0000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000c48656c6c6f20576f726c64210000000000000000000000000000000000000000".into()));

        let result: String = {
            let token = contract(&transport);

            // when
            futures::executor::block_on(token.query(
                "name",
                (),
                Address::from_low_u64_be(5),
                Options::with(|options| {
                    options.gas_price = Some(10_000_000.into());
                }),
                BlockId::Number(BlockNumber::Latest),
            ))
            .unwrap()
        };

        // then
        transport.assert_request("eth_call", &["{\"data\":\"0x06fdde03\",\"from\":\"0x0000000000000000000000000000000000000005\",\"gasPrice\":\"0x989680\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(), "\"latest\"".into()]);
        transport.assert_no_more_requests();
        assert_eq!(result, "Hello World!".to_owned());
    }

    #[test]
    fn should_call_a_contract_function() {
        // given
        let mut transport = TestTransport::default();
        transport.set_response(rpc::Value::String(format!("{:?}", H256::from_low_u64_be(5))));

        let result = {
            let token = contract(&transport);

            // when
            futures::executor::block_on(token.call("name", (), Address::from_low_u64_be(5), Options::default()))
                .unwrap()
        };

        // then
        transport.assert_request("eth_sendTransaction", &["{\"data\":\"0x06fdde03\",\"from\":\"0x0000000000000000000000000000000000000005\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into()]);
        transport.assert_no_more_requests();
        assert_eq!(result, H256::from_low_u64_be(5));
    }

    #[test]
    fn should_estimate_gas_usage() {
        // given
        let mut transport = TestTransport::default();
        transport.set_response(rpc::Value::String(format!("{:#x}", U256::from(5))));

        let result = {
            let token = contract(&transport);

            // when
            futures::executor::block_on(token.estimate_gas("name", (), Address::from_low_u64_be(5), Options::default()))
                .unwrap()
        };

        // then
        transport.assert_request("eth_estimateGas", &["{\"data\":\"0x06fdde03\",\"from\":\"0x0000000000000000000000000000000000000005\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into()]);
        transport.assert_no_more_requests();
        assert_eq!(result, 5.into());
    }

    #[test]
    fn should_query_single_parameter_function() {
        // given
        let mut transport = TestTransport::default();
        transport.set_response(rpc::Value::String(
            "0x0000000000000000000000000000000000000000000000000000000000000020".into(),
        ));

        let result: U256 = {
            let token = contract(&transport);

            // when
            futures::executor::block_on(token.query(
                "balanceOf",
                Address::from_low_u64_be(5),
                None,
                Options::default(),
                None,
            ))
            .unwrap()
        };

        // then
        transport.assert_request("eth_call", &["{\"data\":\"0x70a082310000000000000000000000000000000000000000000000000000000000000005\",\"to\":\"0x0000000000000000000000000000000000000001\"}".into(), "\"latest\"".into()]);
        transport.assert_no_more_requests();
        assert_eq!(result, 0x20.into());
    }
}
