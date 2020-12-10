use crate::{
    api::Namespace,
    helpers::{self, CallFuture},
    types::{Address, H256},
    Transport,
};

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
    /// Given an address of an account and its password deletes the account from the parity node
    pub fn parity_kill_account(&self, address: &Address, pwd: &str) -> CallFuture<bool, T::Out> {
        let address = helpers::serialize(&address);
        let pwd = helpers::serialize(&pwd);
        CallFuture::new(self.transport.execute("parity_killAccount", vec![address, pwd]))
    }
    /// Imports an account from a given seed/phrase
    /// Retunrs the address of the corresponding seed vinculated account
    pub fn parity_new_account_from_phrase(&self, seed: &str, pwd: &str) -> CallFuture<Address, T::Out> {
        let seed = helpers::serialize(&seed);
        let pwd = helpers::serialize(&pwd);
        CallFuture::new(self.transport.execute("parity_newAccountFromPhrase", vec![seed, pwd]))
    }
    /// Imports an account from a given secret key.
    /// Returns the address of the corresponding Sk vinculated account.
    pub fn new_account_from_secret(&self, secret: &H256, pwd: &str) -> CallFuture<Address, T::Out> {
        let secret = helpers::serialize(&secret);
        let pwd = helpers::serialize(&pwd);
        CallFuture::new(self.transport.execute("parity_newAccountFromSecret", vec![secret, pwd]))
    }
    /// Imports an account from a JSON encoded Wallet file.
    /// Returns the address of the corresponding wallet.
    pub fn parity_new_account_from_wallet(&self, wallet: &str, pwd: &str) -> CallFuture<Address, T::Out> {
        let wallet = helpers::serialize(&wallet);
        let pwd = helpers::serialize(&pwd);
        CallFuture::new(self.transport.execute("parity_newAccountFromWallet", vec![wallet, pwd]))
    }
    /// Removes the address of the Parity node addressbook.
    /// Returns true if the operation suceeded.
    pub fn parity_remove_address(&self, address: &Address) -> CallFuture<bool, T::Out> {
        let address = helpers::serialize(&address);
        CallFuture::new(self.transport.execute("parity_removeAddress", vec![address]))
    }
}

#[cfg(test)]
mod tests {
    use super::ParityAccounts;
    use crate::{api::Namespace, rpc::Value};
    use ethereum_types::{Address, H256};

    rpc_test! (
        ParityAccounts :   parity_kill_account,  &"9b776baeaf3896657a9ba0db5564623b3e0173e0".parse::<Address>().unwrap(), "123456789"
        => "parity_killAccount", vec![r#""0x9b776baeaf3896657a9ba0db5564623b3e0173e0""#, r#""123456789""#];
        Value::Bool(true) => true
    );

    rpc_test! (
        ParityAccounts :   parity_new_account_from_phrase,  "member funny cloth wrist ugly water tuition always fall recycle maze long", "123456789"
        => "parity_newAccountFromPhrase", vec![r#""member funny cloth wrist ugly water tuition always fall recycle maze long""#, r#""123456789""#];
        Value::String("0xE43eD16390bd419d48B09d6E2aa20203D1eF93E1".into()) => "E43eD16390bd419d48B09d6E2aa20203D1eF93E1".parse::<Address>().unwrap()
    );

    rpc_test! (
        ParityAccounts :   new_account_from_secret,  &"c6592108cc3577f6a2d6178bc6947b43db39057195802caa0120f26e39af4945".parse::<H256>().unwrap(), "123456789"
        => "parity_newAccountFromSecret", vec![r#""0xc6592108cc3577f6a2d6178bc6947b43db39057195802caa0120f26e39af4945""#, r#""123456789""#];
        Value::String("0x9b776Baeaf3896657A9ba0db5564623B3E0173e0".into()) => "9b776Baeaf3896657A9ba0db5564623B3E0173e0".parse::<Address>().unwrap()
    );

    rpc_test! (
        ParityAccounts :   parity_new_account_from_wallet,  r#"{"version":3,"id":"3b330c3b-b0b3-4e39-b62e-c2041a98d673","address":"4c8ab9d3e938285776d6717d7319f6a9b1d809dd","Crypto":{"ciphertext":"bb3a6dbf21f0bf2b5eb0b43426590f16650acee9462ab710cca18781691a5739","cipherparams":{"iv":"6a533f77fc5cb8a752a16ec6a3200da1"},"cipher":"aes-128-ctr","kdf":"scrypt","kdfparams":{"dklen":32,"salt":"a58609853dec53c81feb165e346c700e714285771825bb4cbf87c4ea1996b682","n":8192,"r":8,"p":1},"mac":"a71edeb659ed628db13579ce9f75c80c9d386c1239b280548d9a0e58ad20d6c7"}}"#, "123456789"
        => "parity_newAccountFromWallet", vec![r#""{\"version\":3,\"id\":\"3b330c3b-b0b3-4e39-b62e-c2041a98d673\",\"address\":\"4c8ab9d3e938285776d6717d7319f6a9b1d809dd\",\"Crypto\":{\"ciphertext\":\"bb3a6dbf21f0bf2b5eb0b43426590f16650acee9462ab710cca18781691a5739\",\"cipherparams\":{\"iv\":\"6a533f77fc5cb8a752a16ec6a3200da1\"},\"cipher\":\"aes-128-ctr\",\"kdf\":\"scrypt\",\"kdfparams\":{\"dklen\":32,\"salt\":\"a58609853dec53c81feb165e346c700e714285771825bb4cbf87c4ea1996b682\",\"n\":8192,\"r\":8,\"p\":1},\"mac\":\"a71edeb659ed628db13579ce9f75c80c9d386c1239b280548d9a0e58ad20d6c7\"}}""#, r#""123456789""#];
        Value::String("0x4C8aB9d3e938285776d6717d7319F6a9B1d809DD".into()) => "4C8aB9d3e938285776d6717d7319F6a9B1d809DD".parse::<Address>().unwrap()
    );

    rpc_test! (
        ParityAccounts :   parity_remove_address,  &"9b776baeaf3896657a9ba0db5564623b3e0173e0".parse::<Address>().unwrap()
        => "parity_removeAddress", vec![r#""0x9b776baeaf3896657a9ba0db5564623b3e0173e0""#];
        Value::Bool(true) => true
    );
}
