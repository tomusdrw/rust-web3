//! Partial implementation of the `Accounts` namespace.

use crate::api::{Namespace, Web3};
use crate::error::Error;
use crate::helpers::CallFuture;
use crate::types::{Address, Bytes, SignedData, SignedTransaction, TransactionData, H256, U256};
use crate::Transport;
use ethsign::{SecretKey, Signature};
use futures::future::{self, Either, FutureResult, Join3};
use futures::{Future, Poll};
use parity_crypto::Keccak256;

/// `Accounts` namespace
#[derive(Debug, Clone)]
pub struct Accounts<T> {
    transport: T,
}

impl<T: Transport> Namespace<T> for Accounts<T> {
    fn new(transport: T) -> Self
    where
        Self: Sized,
    {
        Accounts { transport }
    }

    fn transport(&self) -> &T {
        &self.transport
    }
}

impl<T: Transport> Accounts<T> {
    /// Gets the parent `web3` namespace
    fn web3(&self) -> Web3<T> {
        Web3::new(self.transport.clone())
    }

    /// Signs an Ethereum transaction with a given private key.
    pub fn sign_transaction(&self, tx: TransactionData, key: SecretKey) -> SignTransactionFuture<T> {
        unimplemented!()
    }

    /// Hash a message. The data will be UTF-8 HEX decoded and enveloped as
    /// follows: `"\u{19}Ethereum Signed Message:\n" + message.length + message`
    /// and hashed using keccak256.
    pub fn hash_message<S>(&self, message: S) -> H256
    where
        S: AsRef<str>,
    {
        let message = message.as_ref();
        let eth_message = format!("\u{0019}Ethereum Signed Message:\n{}{}", message.len(), message);

        eth_message.as_bytes().keccak256().into()
    }

    /// Sign arbitrary string data. The data is UTF-8 encoded and enveloped the
    /// same way as with `hash_message`.
    pub fn sign<S>(&self, message: S, key: &SecretKey) -> Result<SignedData, ethsign::Error>
    where
        S: AsRef<str>,
    {
        let message = message.as_ref().to_string();
        let message_hash = self.hash_message(&message);

        let signature = key.sign(&message_hash[..])?;
        // this is what web3.js does ¯\_(ツ)_/¯, it is documented here:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#sign
        let v = signature.v + 27;

        let signature_bytes = Bytes({
            let mut bytes = Vec::with_capacity(65);
            bytes.extend_from_slice(&signature.r[..]);
            bytes.extend_from_slice(&signature.s[..]);
            bytes.push(v);
            bytes
        });

        Ok(SignedData {
            message,
            message_hash: message_hash.into(),
            v,
            r: signature.r.into(),
            s: signature.s.into(),
            signature: signature_bytes,
        })
    }

    /// Recovers the Ethereum address which was used to sign the given data.
    pub fn recover<S, Sig>(&self, message: S, signature: Sig) -> Result<Address, ethsign::Error>
    where
        S: AsRef<str>,
        Sig: IntoSignature,
    {
        let message_hash = self.hash_message(message);
        let signature = signature.into_signature();

        let public_key = signature.recover(&message_hash[..])?;

        Ok(public_key.address().into())
    }
}

type MaybeReady<T, R> = Either<FutureResult<R, Error>, CallFuture<R, <T as Transport>::Out>>;

/// Future resolving when transaction signing is complete
pub struct SignTransactionFuture<T: Transport> {
    accounts: Accounts<T>,
    tx: TransactionData,
    key: SecretKey,
    inner: Join3<MaybeReady<T, U256>, MaybeReady<T, U256>, MaybeReady<T, String>>,
}

impl<T: Transport> SignTransactionFuture<T> {
    /// Creates a new SignTransactionFuture with accounts and transaction data.
    pub fn new(accounts: Accounts<T>, tx: TransactionData, key: SecretKey) -> SignTransactionFuture<T> {
        macro_rules! maybe {
            ($o: expr, $f: expr) => {
                match $o.clone() {
                    Some(value) => Either::A(future::ok(value)),
                    None => Either::B($f),
                }
            };
        }

        let from = key.public().address().into();
        let inner = Future::join3(
            maybe!(tx.nonce, accounts.web3().eth().transaction_count(from, None)),
            maybe!(tx.gas_price, accounts.web3().eth().gas_price()),
            maybe!(None, accounts.web3().net().version()),
        );

        SignTransactionFuture {
            accounts,
            tx,
            key,
            inner,
        }
    }
}

impl<T: Transport> Future for SignTransactionFuture<T> {
    type Item = SignedTransaction;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (nonce, gas_price, chain_id) = try_ready!(self.inner.poll());
        Ok(unimplemented!())
    }
}

pub trait IntoSignature {
    fn into_signature(self) -> Signature;
}

impl IntoSignature for (u8, H256, H256) {
    fn into_signature(self) -> Signature {
        let (v, r, s) = self;
        Signature {
            v: v - 27, // ¯\_(ツ)_/¯
            r: r.into(),
            s: s.into(),
        }
    }
}

impl<'a> IntoSignature for &'a Bytes {
    fn into_signature(self) -> Signature {
        if self.0.len() != 65 {
            panic!("invalid signature bytes");
        }

        let v = self.0[64];
        let r = H256::from_slice(&self.0[0..32]);
        let s = H256::from_slice(&self.0[32..64]);

        (v, r, s).into_signature()
    }
}

impl<'a> IntoSignature for &'a SignedData {
    fn into_signature(self) -> Signature {
        (self.v, self.r, self.s).into_signature()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tests::TestTransport;
    use crate::types::{Bytes, SignedData};
    use ethsign::{SecretKey, Signature};
    use rustc_hex::FromHex;

    #[test]
    fn hash_message() {
        // test vector taken from:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#hashmessage

        let accounts = Accounts::new(TestTransport::default());
        let hash = accounts.hash_message("Hello World");

        assert_eq!(
            hash,
            "a1de988600a42c4b4ab089b619297c17d53cffae5d5120d82d8a92d0bb3b78f2"
                .parse()
                .unwrap()
        );
    }

    #[test]
    fn sign() {
        // test vector taken from:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#sign

        let accounts = Accounts::new(TestTransport::default());

        let secret: Vec<u8> = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .from_hex()
            .unwrap();
        let key = SecretKey::from_raw(&secret).unwrap();
        let signed = accounts.sign("Some data", &key).unwrap();

        assert_eq!(
            signed.message_hash,
            "1da44b586eb0729ff70a73c326926f6ed5a25f5b056e7f47fbc6e58d86871655"
                .parse()
                .unwrap()
        );
        assert_eq!(
            signed.signature.0,
            "b91467e570a6466aa9e9876cbcd013baba02900b8979d43fe208a4a4f339f5fd6007e74cd82e037b800186422fc2da167c747ef045e5d18a5f5d4300f8e1a0291c"
                .from_hex::<Vec<u8>>()
                .unwrap()
        );
    }

    #[test]
    fn recover() {
        // test vector taken from:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#recover

        let accounts = Accounts::new(TestTransport::default());

        let v = 0x1cu8;
        let r: H256 = "b91467e570a6466aa9e9876cbcd013baba02900b8979d43fe208a4a4f339f5fd"
            .parse()
            .unwrap();
        let s: H256 = "6007e74cd82e037b800186422fc2da167c747ef045e5d18a5f5d4300f8e1a029"
            .parse()
            .unwrap();

        assert_eq!(
            accounts.recover("Some data", (v, r, s)).unwrap(),
            "2c7536E3605D9C16a7a3D7b1898e529396a65c23".parse().unwrap()
        );
    }

    #[test]
    fn into_signature() {
        let v = 0x1cu8;
        let r: H256 = "b91467e570a6466aa9e9876cbcd013baba02900b8979d43fe208a4a4f339f5fd"
            .parse()
            .unwrap();
        let s: H256 = "6007e74cd82e037b800186422fc2da167c747ef045e5d18a5f5d4300f8e1a029"
            .parse()
            .unwrap();

        let signed = SignedData {
            message: "Some data".to_string(),
            message_hash: "1da44b586eb0729ff70a73c326926f6ed5a25f5b056e7f47fbc6e58d86871655".parse().unwrap(),
            v,
            r,
            s,
    signature: Bytes("b91467e570a6466aa9e9876cbcd013baba02900b8979d43fe208a4a4f339f5fd6007e74cd82e037b800186422fc2da167c747ef045e5d18a5f5d4300f8e1a0291c"
                .from_hex::<Vec<u8>>()
                .unwrap()),
        };
        let expected_signature = Signature {
            v: 0x01,
            r: r.into(),
            s: s.into(),
        };

        assert_eq!(signed.into_signature(), expected_signature);
        assert_eq!((v, r, s).into_signature(), expected_signature);
        assert_eq!(signed.signature.into_signature(), expected_signature);
    }
}
