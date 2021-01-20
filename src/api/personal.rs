//! `Personal` namespace

use crate::{
    api::Namespace,
    helpers::{self, CallFuture},
    types::{Address, RawTransaction, TransactionRequest, H256},
    Transport,
};

/// `Personal` namespace
#[derive(Debug, Clone)]
pub struct Personal<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for Personal<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        Personal { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Personal<T> {
    /// Returns a list of available accounts.
    pub fn list_accounts(&self) -> CallFuture<Vec<Address>, T::Out> {
        CallFuture::new(self.transport.execute("personal_listAccounts", vec![]))
    }

    /// Creates a new account and protects it with given password.
    /// Returns the address of created account.
    pub fn new_account(&self, password: &str) -> CallFuture<Address, T::Out> {
        let password = helpers::serialize(&password);
        CallFuture::new(self.transport.execute("personal_newAccount", vec![password]))
    }

    /// Unlocks the account with given password for some period of time (or single transaction).
    /// Returns `true` if the call was successful.
    pub fn unlock_account(&self, address: Address, password: &str, duration: Option<u16>) -> CallFuture<bool, T::Out> {
        let address = helpers::serialize(&address);
        let password = helpers::serialize(&password);
        let duration = helpers::serialize(&duration);
        CallFuture::new(
            self.transport
                .execute("personal_unlockAccount", vec![address, password, duration]),
        )
    }

    /// Sends a transaction from locked account.
    /// Returns transaction hash.
    pub fn send_transaction(&self, transaction: TransactionRequest, password: &str) -> CallFuture<H256, T::Out> {
        let transaction = helpers::serialize(&transaction);
        let password = helpers::serialize(&password);
        CallFuture::new(
            self.transport
                .execute("personal_sendTransaction", vec![transaction, password]),
        )
    }

    /// Signs a transaction without dispatching it to the network.
    /// The account does not need to be unlocked to make this call, and will not be left unlocked after.
    /// Returns a signed transaction in raw bytes along with it's details.
    pub fn sign_transaction(
        &self,
        transaction: TransactionRequest,
        password: &str,
    ) -> CallFuture<RawTransaction, T::Out> {
        let transaction = helpers::serialize(&transaction);
        let password = helpers::serialize(&password);
        CallFuture::new(
            self.transport
                .execute("personal_signTransaction", vec![transaction, password]),
        )
    }

    /// Imports a raw key and protects it with the given password.
    /// Returns the address of created account.
    pub fn import_raw_key(&self, private_key: &[u8; 32], password: &str) -> CallFuture<Address, T::Out> {
        let private_key = hex::encode(private_key);
        let private_key = helpers::serialize(&private_key);
        let password = helpers::serialize(&password);

        CallFuture::new(
            self.transport
                .execute("personal_importRawKey", vec![private_key, password]),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::Personal;
    use crate::{
        api::Namespace,
        rpc::Value,
        types::{Address, RawTransaction, TransactionRequest},
    };
    use hex_literal::hex;

    const EXAMPLE_TX: &str = r#"{
    "raw": "0xd46e8dd67c5d32be8d46e8dd67c5d32be8058bb8eb970870f072445675058bb8eb970870f072445675",
    "tx": {
      "hash": "0xc6ef2fc5426d6ad6fd9e2a26abeab0aa2411b7ab17f30a99d3cb96aed1d1055b",
      "nonce": "0x0",
      "blockHash": "0xbeab0aa2411b7ab17f30a99d3cb9c6ef2fc5426d6ad6fd9e2a26a6aed1d1055b",
      "blockNumber": "0x15df",
      "transactionIndex": "0x1",
      "from": "0x407d73d8a49eeb85d32cf465507dd71d507100c1",
      "to": "0x853f43d8a49eeb85d32cf465507dd71d507100c1",
      "value": "0x7f110",
      "gas": "0x7f110",
      "gasPrice": "0x09184e72a000",
      "input": "0x603880600c6000396000f300603880600c6000396000f3603880600c6000396000f360"
    }
  }"#;

    rpc_test! (
      Personal:list_accounts => "personal_listAccounts";
      Value::Array(vec![Value::String("0x0000000000000000000000000000000000000123".into())]) => vec![Address::from_low_u64_be(0x123)]
    );

    rpc_test! (
      Personal:new_account, "hunter2" => "personal_newAccount", vec![r#""hunter2""#];
      Value::String("0x0000000000000000000000000000000000000123".into()) => Address::from_low_u64_be(0x123)
    );

    rpc_test! (
      Personal:unlock_account, Address::from_low_u64_be(0x123), "hunter2", None
      =>
      "personal_unlockAccount", vec![r#""0x0000000000000000000000000000000000000123""#, r#""hunter2""#, r#"null"#];
      Value::Bool(true) => true
    );

    rpc_test! (
      Personal:send_transaction, TransactionRequest {
        from: Address::from_low_u64_be(0x123), to: Some(Address::from_low_u64_be(0x123)),
        gas: None, gas_price: Some(0x1.into()),
        value: Some(0x1.into()), data: None,
        nonce: None, condition: None,
      }, "hunter2"
      =>
      "personal_sendTransaction", vec![r#"{"from":"0x0000000000000000000000000000000000000123","gasPrice":"0x1","to":"0x0000000000000000000000000000000000000123","value":"0x1"}"#, r#""hunter2""#];
      Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()) => Address::from_low_u64_be(0x123)
    );

    rpc_test! (
      Personal:sign_transaction, TransactionRequest {
        from: hex!("407d73d8a49eeb85d32cf465507dd71d507100c1").into(),
        to: Some(hex!("853f43d8a49eeb85d32cf465507dd71d507100c1").into()),
        gas: Some(0x7f110.into()),
        gas_price: Some(0x09184e72a000u64.into()),
        value: Some(0x7f110.into()),
        data: Some(hex!("603880600c6000396000f300603880600c6000396000f3603880600c6000396000f360").into()),
        nonce: Some(0x0.into()),
        condition: None,
      }, "hunter2"
      =>
      "personal_signTransaction", vec![r#"{"data":"0x603880600c6000396000f300603880600c6000396000f3603880600c6000396000f360","from":"0x407d73d8a49eeb85d32cf465507dd71d507100c1","gas":"0x7f110","gasPrice":"0x9184e72a000","nonce":"0x0","to":"0x853f43d8a49eeb85d32cf465507dd71d507100c1","value":"0x7f110"}"#, r#""hunter2""#];
      ::serde_json::from_str(EXAMPLE_TX).unwrap()
      => ::serde_json::from_str::<RawTransaction>(EXAMPLE_TX).unwrap()
    );

    rpc_test! {
      Personal:import_raw_key, &[0u8; 32], "hunter2" =>
      "personal_importRawKey", vec![r#""0000000000000000000000000000000000000000000000000000000000000000""#, r#""hunter2""#];
      Value::String("0x0000000000000000000000000000000000000123".into()) => Address::from_low_u64_be(0x123)
    }
}
