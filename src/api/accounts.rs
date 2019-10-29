//! Partial implementation of the `Accounts` namespace.

use crate::api::Namespace;
use crate::types::{Bytes, SignedData, H256};
use crate::Transport;
use ethsign::SecretKey;
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
    fn hash_message<S>(&self, message: S) -> H256
    where
        S: AsRef<str>,
    {
        let message = message.as_ref();
        let eth_message = format!("\u{0019}Ethereum Signed Message:\n{}{}", message.len(), message);

        eth_message.as_bytes().keccak256().into()
    }

    fn sign<S>(&self, message: S, key: &SecretKey) -> Result<SignedData, ethsign::Error>
    where
        S: AsRef<str>,
    {
        let message = message.as_ref().to_string();
        let message_hash = self.hash_message(&message);

        let signature = key.sign(&message_hash[..])?;
        let v = signature.v + 27; // this is what web3.js does ¯\_(ツ)_/¯
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
            r: signature.r,
            s: signature.s,
            signature: signature_bytes,
        })
    }
}

pub trait IntoSignature {
    fn into_signature(self) -> (u8, [u8; 32], [u8; 32]);
}

impl IntoSignature for (u8, [u8; 32], [u8; 32]) {
    fn into_signature(self) -> (u8, [u8; 32], [u8; 32]) {
        self
    }
}

impl IntoSignature for Bytes {
    fn into_signature(self) -> (u8, [u8; 32], [u8; 32]) {
        if self.0.len() != 65 {
            panic!("invalid signature bytes");
        }

        unimplemented!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tests::TestTransport;
    use ethsign::SecretKey;
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

        let secret: Vec<u8> = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .from_hex()
            .unwrap();
        let key = SecretKey::from_raw(&secret).unwrap();

        let accounts = Accounts::new(TestTransport::default());
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
}
