//! Contract deployment utilities

use ethabi;
use futures::{Async, Future, Poll};
use rustc_hex::{ToHex, FromHex};
use std::{collections::HashMap, time};

use crate::api::{Eth, Namespace};
use crate::confirm;
use crate::contract::tokens::Tokenize;
use crate::contract::{Contract, Options};
use crate::types::{Address, Bytes, TransactionRequest};
use crate::Transport;

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
    pub fn execute<P, V>(self, code: V, params: P, from: Address) -> Result<PendingContract<T>, ethabi::Error>
    where
        P: Tokenize,
        V: AsRef<str>,
    {
        let options = self.options;
        let eth = self.eth;
        let abi = self.abi;

        let mut code_hex = code.as_ref().to_string();

        for (lib, address) in self.linker {
            if lib.len() > 38 {
                return Err(ethabi::ErrorKind::Msg(String::from("The library name should be under 39 characters.")).into());
            }
            let replace = format!("__{:_<38}", lib); // This makes the required width 38 characters and will pad with `_` to match it. 
            let address: String = address.as_ref().to_hex();
            code_hex = code_hex.replacen(&replace, &address, 1);
        }
        code_hex = code_hex.replace("\"", "").replace("0x", ""); // This is to fix truffle + serde_json redundant `"` and `0x`
        let code = code_hex.from_hex().map_err(|e| ethabi::ErrorKind::Hex(e))?;

        let params = params.into_tokens();
        let data = match (abi.constructor(), params.is_empty()) {
            (None, false) => return Err(ethabi::ErrorKind::Msg(format!("Constructor is not defined in the ABI.")).into()),
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
        };

        let waiting = confirm::send_transaction_with_confirmation(
            eth.transport().clone(),
            tx,
            self.poll_interval,
            self.confirmations,
        );

        Ok(PendingContract {
            eth: Some(eth),
            abi: Some(abi),
            waiting,
        })
    }
}

/// Contract being deployed.
pub struct PendingContract<T: Transport> {
    eth: Option<Eth<T>>,
    abi: Option<ethabi::Contract>,
    waiting: confirm::SendTransactionWithConfirmation<T>,
}

impl<T: Transport> Future for PendingContract<T> {
    type Item = Contract<T>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let receipt = try_ready!(self.waiting.poll());
        let eth = self.eth.take().expect("future polled after ready; qed");
        let abi = self.abi.take().expect("future polled after ready; qed");

        match receipt.contract_address {
            Some(address) => Ok(Async::Ready(Contract::new(eth, address, abi))),
            None => Err(Error::ContractDeploymentFailure(receipt.transaction_hash).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::api::{self, Namespace};
    use crate::contract::{Contract, Options};
    use crate::helpers::tests::TestTransport;
    use crate::rpc;
    use crate::types::U256;
    use futures::Future;
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
        "{\"blockHash\":\"0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c54878f\",\"blockNumber\":\"0x256\",\"contractAddress\":\"0x600515dfe465f600f0c9793fa27cd2794f3ec0e1\",\"cumulativeGasUsed\":\"0xe57e0\",\"gasUsed\":\"0xe57e0\",\"logs\":[],\"logsBloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"root\":null,\"transactionHash\":\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\",\"transactionIndex\":\"0x0\"}"
      ).unwrap();
        transport.add_response(receipt.clone());
        // block number
        transport.add_response(rpc::Value::String("0x25a".into()));
        // receipt again
        transport.add_response(receipt);

        {
            let builder = Contract::deploy(api::Eth::new(&transport), include_bytes!("./res/token.json")).unwrap();

            // when
            builder
                .options(Options::with(|opt| opt.value = Some(5.into())))
                .confirmations(1)
                .execute(
                    vec![1, 2, 3, 4],
                    (U256::from(1_000_000), "My Token".to_owned(), 3u64, "MT".to_owned()),
                    5.into(),
                )
                .unwrap()
                .wait()
                .unwrap();
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
        use serde_json::to_vec;
        let mut transport = TestTransport::default();
        let receipt = ::serde_json::from_str::<rpc::Value>(
        "{\"blockHash\":\"0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c54878f\",\"blockNumber\":\"0x256\",\"contractAddress\":\"0x600515dfe465f600f0c9793fa27cd2794f3ec0e1\",\"cumulativeGasUsed\":\"0xe57e0\",\"gasUsed\":\"0xe57e0\",\"logs\":[],\"logsBloom\":\"0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000\",\"root\":null,\"transactionHash\":\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\",\"transactionIndex\":\"0x0\"}"
        ).unwrap();

        for _ in 0..2 {
            transport.add_response(rpc::Value::String("0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1".into()));
            transport.add_response(rpc::Value::String("0x0".into()));
            transport.add_response(rpc::Value::Array(vec![rpc::Value::String("0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c54878f".into())]));
            transport.add_response(rpc::Value::Array(vec![rpc::Value::String("0xd5311584a9867d8e129113e1ec9db342771b94bd4533aeab820a5bcc2c548790".into())]));
            transport.add_response(receipt.clone());
            transport.add_response(rpc::Value::String("0x25a".into()));
            transport.add_response(receipt.clone());
        }

        let lib: Value = serde_json::from_slice(include_bytes!("./res/MyLibrary.json")).unwrap();
        let lib_abi: Vec<u8> = to_vec(&lib["abi"]).unwrap();
        let lib_code: Vec<u8> = to_vec(&lib["bytecode"]).unwrap();

        let main: Value = serde_json::from_slice(include_bytes!("./res/Main.json")).unwrap();
        let main_abi: Vec<u8> = to_vec(&main["abi"]).unwrap();
        let main_code: Vec<u8> = to_vec(&main["bytecode"]).unwrap();

        let lib_address;
        {
            let builder = Contract::deploy(api::Eth::new(&transport), &lib_abi).unwrap();
            lib_address = builder.execute(lib_code, (), 0.into()).unwrap().wait().unwrap().address();
        }

        transport.assert_request("eth_sendTransaction", &[
            "{\"data\":\"0x3630616436313030326636303062383238323832333938303531363030303161363037333134363030303831313436313030316635373631303032313536356266653562353033303630303035323630373338313533383238316633666537333030303030303030303030303030303030303030303030303030303030303030303030303030303033303134363038303630343035323630303433363130363035363537363030303335376330313030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030393030343633666666666666666631363830363366386138666436643134363035623537356236303030383066643562363036313630373735363562363034303531383038323831353236303230303139313530353036303430353138303931303339306633356236303030363130313030393035303930353666656131363536323761376137323330353832306235303039316164636237656639393837646438646161363635636563353732383031626638323433353330643730643532363331663964356464623934336530303239\",\"from\":\"0x0000000000000000000000000000000000000000\"}"
            .into()]);
        transport.assert_request("eth_newBlockFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request("eth_getTransactionReceipt", &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()]);
        transport.assert_request("eth_blockNumber", &[]);
        transport.assert_request("eth_getTransactionReceipt", &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()]);
        transport.assert_no_more_requests();
        {
            let builder = Contract::deploy_from_truffle(api::Eth::new(&transport), &main_abi, {
                let mut linker = HashMap::new();
                linker.insert("MyLibrary", lib_address);
                linker
            })
            .unwrap();
            let _ = builder.execute(main_code, (), 0.into()).unwrap().wait().unwrap();
        }

        transport.assert_request("eth_sendTransaction", &[
            "{\"data\":\"0x363038303630343035323334383031353631303031303537363030303830666435623530363130313366383036313030323036303030333936303030663366653630383036303430353236303034333631303631303034313537363030303335376330313030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030393030343633666666666666666631363830363366386138666436643134363130303436353735623630303038306664356233343830313536313030353235373630303038306664356235303631303035623631303037313536356236303430353138303832383135323630323030313931353035303630343035313830393130333930663335623630303037333630303531356466653436356636303066306339373933666132376364323739346633656330653136336638613866643664363034303531383136336666666666666666313637633031303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030303030323831353236303034303136303230363034303531383038333033383138363830336231353830313536313030643335373630303038306664356235303561663431353830313536313030653735373364363030303830336533643630303066643562353035303530353036303430353133643630323038313130313536313030666435373630303038306664356238313031393038303830353139303630323030313930393239313930353035303530393035303930353666656131363536323761376137323330353832303538306433373736623364313332313432663433316531343161326532306264346464343930376661333034666565613762363034653866333965643539353230303239\",\"from\":\"0x0000000000000000000000000000000000000000\"}"
            .into()]);

        transport.assert_request("eth_newBlockFilter", &[]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request("eth_getFilterChanges", &["\"0x0\"".into()]);
        transport.assert_request("eth_getTransactionReceipt", &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()]);
        transport.assert_request("eth_blockNumber", &[]);
        transport.assert_request("eth_getTransactionReceipt", &["\"0x70ae45a5067fdf3356aa615ca08d925a38c7ff21b486a61e79d5af3969ebc1a1\"".into()]);
        transport.assert_no_more_requests();
    }

}
