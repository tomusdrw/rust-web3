//! Contract deployment utilities

use crate::{
    api::{Eth, Namespace},
    confirm,
    contract::{tokens::Tokenize, Contract, Options},
    error,
    types::{Address, Bytes, TransactionReceipt, TransactionRequest},
    Transport,
};
#[cfg(feature = "signing")]
use crate::{signing::Key, types::TransactionParameters};
use futures::{Future, TryFutureExt};
use std::{collections::HashMap, time};

pub use crate::contract::error::deploy::Error;

/// A configuration builder for contract deployment.
#[derive(Debug)]
pub struct Builder<T: Transport> {
    pub(crate) eth: Eth<T>,
    pub(crate) abi: ethabi::Contract,
    pub(crate) options: Options,
    pub(crate) confirmations: usize,
    pub(crate) poll_interval: time::Duration,
    pub(crate) linker: HashMap<String, Address>,
}

impl<T: Transport> Builder<T> {
    /// Number of confirmations required after code deployment.
    pub fn confirmations(mut self, confirmations: usize) -> Self {
        self.confirmations = confirmations;
        self
    }

    /// Deployment transaction options.
    pub fn options(mut self, options: Options) -> Self {
        self.options = options;
        self
    }

    /// Confirmations poll interval.
    pub fn poll_interval(mut self, interval: time::Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    /// Execute deployment passing code and contructor parameters.
    pub async fn execute<P, V>(self, code: V, params: P, from: Address) -> Result<Contract<T>, Error>
    where
        P: Tokenize,
        V: AsRef<str>,
    {
        let transport = self.eth.transport().clone();
        let poll_interval = self.poll_interval;
        let confirmations = self.confirmations;

        self.do_execute(code, params, from, move |tx| {
            confirm::send_transaction_with_confirmation(transport, tx, poll_interval, confirmations)
        })
        .await
    }
    /// Execute deployment passing code and constructor parameters.
    ///
    /// Unlike the above `execute`, this method uses
    /// `sign_raw_transaction_with_confirmation` instead of
    /// `sign_transaction_with_confirmation`, which requires the account from
    /// which the transaction is sent to be unlocked.
    pub async fn sign_and_execute<P, V>(
        self,
        code: V,
        params: P,
        from: Address,
        password: &str,
    ) -> Result<Contract<T>, Error>
    where
        P: Tokenize,
        V: AsRef<str>,
    {
        let transport = self.eth.transport().clone();
        let poll_interval = self.poll_interval;
        let confirmations = self.confirmations;

        self.do_execute(code, params, from, move |tx| {
            crate::api::Personal::new(transport.clone())
                .sign_transaction(tx, password)
                .and_then(move |signed_tx| {
                    confirm::send_raw_transaction_with_confirmation(
                        transport,
                        signed_tx.raw,
                        poll_interval,
                        confirmations,
                    )
                })
        })
        .await
    }

    /// Execute deployment passing code and constructor parameters.
    ///
    /// Unlike the above `sign_and_execute`, this method allows the
    /// caller to pass in a private key to sign the transaction with
    /// and therefore allows deploying from an account that the
    /// ethereum node doesn't need to know the private key for.
    ///
    /// An optional `chain_id` parameter can be passed to provide
    /// replay protection for transaction signatures. Passing `None`
    /// would create a transaction WITHOUT replay protection and
    /// should be avoided.
    /// You can obtain `chain_id` of the network you are connected
    /// to using `web3.eth().chain_id()` method.
    #[cfg(feature = "signing")]
    pub async fn sign_with_key_and_execute<P, V, K>(
        self,
        code: V,
        params: P,
        from: K,
        chain_id: Option<u64>,
    ) -> Result<Contract<T>, Error>
    where
        P: Tokenize,
        V: AsRef<str>,
        K: Key,
    {
        let transport = self.eth.transport().clone();
        let poll_interval = self.poll_interval;
        let confirmations = self.confirmations;

        self.do_execute(code, params, from.address(), move |tx| async move {
            let tx = TransactionParameters {
                nonce: tx.nonce,
                to: tx.to,
                gas: tx.gas.unwrap_or(1_000_000.into()),
                gas_price: tx.gas_price,
                value: tx.value.unwrap_or(0.into()),
                data: tx
                    .data
                    .expect("Tried to deploy a contract but transaction data wasn't set"),
                chain_id,
                transaction_type: tx.transaction_type,
                access_list: tx.access_list,
            };
            let signed_tx = crate::api::Accounts::new(transport.clone())
                .sign_transaction(tx, from)
                .await?;
            confirm::send_raw_transaction_with_confirmation(
                transport,
                signed_tx.raw_transaction,
                poll_interval,
                confirmations,
            )
            .await
        })
        .await
    }

    async fn do_execute<P, V, Ft>(
        self,
        code: V,
        params: P,
        from: Address,
        send: impl FnOnce(TransactionRequest) -> Ft,
    ) -> Result<Contract<T>, Error>
    where
        P: Tokenize,
        V: AsRef<str>,
        Ft: Future<Output = error::Result<TransactionReceipt>>,
    {
        let options = self.options;
        let eth = self.eth;
        let abi = self.abi;

        let mut code_hex = code.as_ref().to_string();

        for (lib, address) in self.linker {
            if lib.len() > 38 {
                return Err(Error::Abi(ethabi::Error::InvalidName(
                    "The library name should be under 39 characters.".into(),
                )));
            }
            let replace = format!("__{:_<38}", lib); // This makes the required width 38 characters and will pad with `_` to match it.
            let address: String = hex::encode(address);
            code_hex = code_hex.replacen(&replace, &address, 1);
        }
        code_hex = code_hex.replace("\"", "").replace("0x", ""); // This is to fix truffle + serde_json redundant `"` and `0x`
        let code =
            hex::decode(&code_hex).map_err(|e| ethabi::Error::InvalidName(format!("hex decode error: {}", e)))?;

        let params = params.into_tokens();
        let data = match (abi.constructor(), params.is_empty()) {
            (None, false) => {
                return Err(Error::Abi(ethabi::Error::InvalidName(
                    "Constructor is not defined in the ABI.".into(),
                )));
            }
            (None, true) => code,
            (Some(constructor), _) => constructor.encode_input(code, &params)?,
        };

        let tx = TransactionRequest {
            from,
            to: None,
            gas: options.gas,
            gas_price: options.gas_price,
            value: options.value,
            nonce: options.nonce,
            data: Some(Bytes(data)),
            condition: options.condition,
            transaction_type: options.transaction_type,
            access_list: options.access_list,
        };
        let receipt = send(tx).await?;
        match receipt.status {
            Some(status) if status == 0.into() => Err(Error::ContractDeploymentFailure(receipt.transaction_hash)),
            // If the `status` field is not present we use the presence of `contract_address` to
            // determine if deployment was successfull.
            _ => match receipt.contract_address {
                Some(address) => Ok(Contract::new(eth, address, abi)),
                None => Err(Error::ContractDeploymentFailure(receipt.transaction_hash)),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        api::{self, Namespace},
        contract::{Contract, Options},
        rpc,
        transports::test::TestTransport,
        types::{Address, U256},
    };
    use serde_json::Value;
    use std::collections::HashMap;

    #[test]
    fn should_deploy_a_contract() {
        // given
        let mut transport = TestTransport::default();
        // Transaction Hash
        transport.add_response(rpc::Value::String(
            "0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1".into(),
        ));
        // BlockFilter
        transport.add_response(rpc::Value::String("0x0".into()));
        // getFilterChanges
        transport.add_response(rpc::Value::Array(vec![rpc::Value::String(
            "0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c54878f".into(),
        )]));
        transport.add_response(rpc::Value::Array(vec![rpc::Value::String(
            "0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c548790".into(),
        )]));
        // receipt
        let receipt = ::serde_json::from_str::<rpc::Value>(
        "{\"blockHash\":\"0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c54878f\",\"blockNumber\":\"0x256\",\"contractAddress\":\"0x600515dfe465f600f0c9793fa27cd2794f3ec0e1\",\"cumulativeGasUsed\":\"0xe57e0\",\"gasUsed\":\"0xe57e0\",\"logs\":[],\"logsBloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"root\":null,\"transactionHash\":\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\",\"transactionIndex\":\"0x0\", \"status\": \"0x1\"}"
      ).unwrap();
        transport.add_response(receipt.clone());
        // block number
        transport.add_response(rpc::Value::String("0x25a".into()));
        // receipt again
        transport.add_response(receipt);

        {
            let builder = Contract::deploy(api::Eth::new(&transport), include_bytes!("./res/token.json")).unwrap();

            // when
            futures::executor::block_on(
                builder
                    .options(Options::with(|opt| opt.value = Some(5.into())))
                    .confirmations(1)
                    .execute(
                        "0x01020304",
                        (U256::from(1_000_000), "My Token".to_owned(), 3u64, "MT".to_owned()),
                        Address::from_low_u64_be(5),
                    ),
            )
            .unwrap()
        };

        // then
        transport.assert_request("eth_sendTransaction", &[
      "{\"data\":\"0x0102030400000000000000000000000000000000000000000000000000000000000f42400000000000000000000000000000000000000000000000000000000000000080000000000000000000000000000000000000000000000000000000000000000300000000000000000000000000000000000000000000000000000000000000c000000000000000000000000000000000000000000000000000000000000000084d7920546f6b656e00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000024d54000000000000000000000000000000000000000000000000000000000000\",\"from\":\"0x0000000000000000000000000000000000000005\",\"value\":\"0x5\"}".into(),
    ]);
        transport.assert_request("eth_newBlockFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()],
        );
        transport.assert_request("eth_blockNumber", &[]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()],
        );
        transport.assert_no_more_requests();
    }

    #[test]
    fn deploy_linked_contract() {
        use serde_json::{to_string, to_vec};
        let mut transport = TestTransport::default();
        let receipt = ::serde_json::from_str::<rpc::Value>(
        "{\"blockHash\":\"0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c54878f\",\"blockNumber\":\"0x256\",\"contractAddress\":\"0x600515dfe465f600f0c9793fa27cd2794f3ec0e1\",\"cumulativeGasUsed\":\"0xe57e0\",\"gasUsed\":\"0xe57e0\",\"logs\":[],\"logsBloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"root\":null,\"transactionHash\":\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\",\"transactionIndex\":\"0x0\", \"status\": \"0x1\"}"
        ).unwrap();

        for _ in 0..2 {
            transport.add_response(rpc::Value::String(
                "0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1".into(),
            ));
            transport.add_response(rpc::Value::String("0x0".into()));
            transport.add_response(rpc::Value::Array(vec![rpc::Value::String(
                "0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c54878f".into(),
            )]));
            transport.add_response(rpc::Value::Array(vec![rpc::Value::String(
                "0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c548790".into(),
            )]));
            transport.add_response(receipt.clone());
            transport.add_response(rpc::Value::String("0x25a".into()));
            transport.add_response(receipt.clone());
        }

        let lib: Value = serde_json::from_slice(include_bytes!("./res/MyLibrary.json")).unwrap();
        let lib_abi: Vec<u8> = to_vec(&lib["abi"]).unwrap();
        let lib_code = to_string(&lib["bytecode"]).unwrap();

        let main: Value = serde_json::from_slice(include_bytes!("./res/Main.json")).unwrap();
        let main_abi: Vec<u8> = to_vec(&main["abi"]).unwrap();
        let main_code = to_string(&main["bytecode"]).unwrap();

        let lib_address;
        {
            let builder = Contract::deploy(api::Eth::new(&transport), &lib_abi).unwrap();
            lib_address = futures::executor::block_on(builder.execute(lib_code, (), Address::zero()))
                .unwrap()
                .address();
        }

        transport.assert_request("eth_sendTransaction", &[
            "{\"data\":\"0x60ad61002f600b82828239805160001a6073146000811461001f57610021565bfe5b5030600052607381538281f3fe73000000000000000000000000000000000000000030146080604052600436106056576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff168063f8a8fd6d14605b575b600080fd5b60616077565b6040518082815260200191505060405180910390f35b600061010090509056fea165627a7a72305820b50091adcb7ef9987dd8daa665cec572801bf8243530d70d52631f9d5ddb943e0029\",\"from\":\"0x0000000000000000000000000000000000000000\"}"
            .into()]);
        transport.assert_request("eth_newBlockFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()],
        );
        transport.assert_request("eth_blockNumber", &[]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()],
        );
        transport.assert_no_more_requests();
        {
            let builder = Contract::deploy_from_truffle(api::Eth::new(&transport), &main_abi, {
                let mut linker = HashMap::new();
                linker.insert("MyLibrary", lib_address);
                linker
            })
            .unwrap();
            let _ = futures::executor::block_on(builder.execute(main_code, (), Address::zero())).unwrap();
        }

        transport.assert_request("eth_sendTransaction", &[
            "{\"data\":\"0x608060405234801561001057600080fd5b5061013f806100206000396000f3fe608060405260043610610041576000357c0100000000000000000000000000000000000000000000000000000000900463ffffffff168063f8a8fd6d14610046575b600080fd5b34801561005257600080fd5b5061005b610071565b6040518082815260200191505060405180910390f35b600073600515dfe465f600f0c9793fa27cd2794f3ec0e163f8a8fd6d6040518163ffffffff167c010000000000000000000000000000000000000000000000000000000002815260040160206040518083038186803b1580156100d357600080fd5b505af41580156100e7573d6000803e3d6000fd5b505050506040513d60208110156100fd57600080fd5b810190808051906020019092919050505090509056fea165627a7a72305820580d3776b3d132142f431e141a2e20bd4dd4907fa304feea7b604e8f39ed59520029\",\"from\":\"0x0000000000000000000000000000000000000000\"}"
            .into()]);

        transport.assert_request("eth_newBlockFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()],
        );
        transport.assert_request("eth_blockNumber", &[]);
        transport.assert_request(
            "eth_getTransactionReceipt",
            &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()],
        );
        transport.assert_no_more_requests();
    }
}
