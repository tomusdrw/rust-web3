use api::Namespace;
use helpers::{self, CallFuture};
use types::{Address, H256};
use Transport;

/// `Parity_Accounts` namespace
#[derive(Debug, Clone)]
pub struct ParityAccounts<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for ParityAccounts<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        ParityAccounts { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> ParityAccounts<T> {
    // Given an address of an account and its password
    // Deletes the account from the parity node
    pub fn parity_kill_account(&self, address: &Address, pwd: &str) -> CallFuture<bool, T::Out> {
       let address = helpers::serialize(&address);
       let pwd = helpers::serialize(&pwd);
       CallFuture::new(
           self.transport
            .execute("parity_killAccount", vec![address, pwd]),
       ) 
    }
    // Imports an account from a given seed/phrase
    // Retunrs the address of the corresponding seed vinculated account
    pub fn parity_new_account_from_phrase(&self, seed: &str, pwd: &str) -> CallFuture<Address, T::Out> {
       let seed = helpers::serialize(&seed);
       let pwd = helpers::serialize(&pwd);
       CallFuture::new(
           self.transport
            .execute("parity_newAccountFromPhrase", vec![seed, pwd]),
       )  
    }
    /// Imports an account from a given secret key.
    /// Returns the address of the corresponding Sk vinculated account.
    pub fn new_account_from_secret(&self, secret: &H256, pwd: &str) -> CallFuture<Address, T::Out> {
       let secret = helpers::serialize(&secret);
       let pwd = helpers::serialize(&pwd);
       CallFuture::new(
           self.transport
            .execute("parity_newAccountFromSecret", vec![secret, pwd]),
       ) 
    }
}

#[cfg(test)]
mod tests {
    use futures::Future;

    use api::Namespace;
    use rpc::Value;
    use ethereum_types::{H256, Address};

    use super::ParityAccounts;

    rpc_test! (
        ParityAccounts :   parity_kill_account,  &Address::from("0x9b776baeaf3896657a9ba0db5564623b3e0173e0"), "123456789"
        => "parity_killAccount", vec![r#""0x9b776baeaf3896657a9ba0db5564623b3e0173e0""#, r#""123456789""#];
        Value::Bool(true) => true
    );

    rpc_test! (
        ParityAccounts :   parity_new_account_from_phrase,  "member funny cloth wrist ugly water tuition always fall recycle maze long", "123456789"
        => "parity_newAccountFromPhrase", vec![r#""member funny cloth wrist ugly water tuition always fall recycle maze long""#, r#""123456789""#];
        Value::String("0xE43eD16390bd419d48B09d6E2aa20203D1eF93E1".into()) => Address::from("0xE43eD16390bd419d48B09d6E2aa20203D1eF93E1")
    );

    rpc_test! (
        ParityAccounts :   new_account_from_secret,  &H256::from("c6592108cc3577f6a2d6178bc6947b43db39057195802caa0120f26e39af4945"), "123456789"
        => "parity_newAccountFromSecret", vec![r#""0xc6592108cc3577f6a2d6178bc6947b43db39057195802caa0120f26e39af4945""#, r#""123456789""#];
        Value::String("0x9b776Baeaf3896657A9ba0db5564623B3E0173e0".into()) => Address::from("0x9b776Baeaf3896657A9ba0db5564623B3E0173e0")
    );
}
