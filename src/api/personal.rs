//! `Personal` namespace

use api::Namespace;
use helpers::{self, CallResult};
use types::{Address, H256, TransactionRequest};

use Transport;

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
    pub fn list_accounts(&self) -> CallResult<Vec<Address>, T::Out> {
        CallResult::new(self.transport.execute("personal_listAccounts", vec![]))
    }

    /// Creates a new account and protects it with given password.
    /// Returns the address of created account.
    pub fn new_account(&self, password: &str) -> CallResult<Address, T::Out> {
        let password = helpers::serialize(&password);
        CallResult::new(
            self.transport
                .execute("personal_newAccount", vec![password]),
        )
    }

    /// Unlocks the account with given password for some period of time (or single transaction).
    /// Returns `true` if the call was successful.
    pub fn unlock_account(&self, address: Address, password: &str, duration: Option<u16>) -> CallResult<bool, T::Out> {
        let address = helpers::serialize(&address);
        let password = helpers::serialize(&password);
        let duration = helpers::serialize(&duration);
        CallResult::new(
            self.transport
                .execute("personal_unlockAccount", vec![address, password, duration]),
        )
    }

    /// Sends a transaction from locked account.
    /// Returns transaction hash.
    pub fn send_transaction(&self, transaction: TransactionRequest, password: &str) -> CallResult<H256, T::Out> {
        let transaction = helpers::serialize(&transaction);
        let password = helpers::serialize(&password);
        CallResult::new(
            self.transport
                .execute("personal_sendTransaction", vec![transaction, password]),
        )
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;

    use api::Namespace;
    use rpc::Value;
    use types::TransactionRequest;

    use super::Personal;

    rpc_test! (
    Personal:list_accounts => "personal_listAccounts";
    Value::Array(vec![Value::String("0x0000000000000000000000000000000000000123".into())]) => vec![0x123.into()]
  );

    rpc_test! (
    Personal:new_account, "hunter2" => "personal_newAccount", vec![r#""hunter2""#];
    Value::String("0x0000000000000000000000000000000000000123".into()) => 0x123
  );

    rpc_test! (
    Personal:unlock_account, 0x123, "hunter2", None
    =>
    "personal_unlockAccount", vec![r#""0x0000000000000000000000000000000000000123""#, r#""hunter2""#, r#"null"#];
    Value::Bool(true) => true
  );

    rpc_test! (
    Personal:send_transaction, TransactionRequest {
      from: 0x123.into(), to: Some(0x123.into()),
      gas: None, gas_price: Some(0x1.into()),
      value: Some(0x1.into()), data: None,
      nonce: None, condition: None,
    }, "hunter2"
    =>
    "personal_sendTransaction", vec![r#"{"from":"0x0000000000000000000000000000000000000123","gasPrice":"0x1","to":"0x0000000000000000000000000000000000000123","value":"0x1"}"#, r#""hunter2""#];
    Value::String("0x0000000000000000000000000000000000000000000000000000000000000123".into()) => 0x123
  );
}
