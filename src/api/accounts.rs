//! Partial implementation of the `Accounts` namespace.

use crate::api::{Namespace, Web3};
use crate::error;
use crate::helpers::CallFuture;
use crate::signing::{self, Signature};
use crate::types::{
    Address, Bytes, Recovery, RecoveryMessage, SignedData, SignedTransaction, TransactionParameters, H256, U256,
};
use crate::Transport;
use futures::future::{self, Either, Join3};
use futures::{
    task::{Context, Poll},
    Future, FutureExt,
};
use rlp::RlpStream;
use std::convert::TryInto;
use std::mem;
use std::pin::Pin;

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
    pub fn sign_transaction<K: signing::Key>(&self, tx: TransactionParameters, key: K) -> SignTransactionFuture<T, K> {
        SignTransactionFuture::new(self, tx, key)
    }

    /// Hash a message according to EIP-191.
    ///
    /// The data is a UTF-8 encoded string and will enveloped as follows:
    /// `"\x19Ethereum Signed Message:\n" + message.length + message` and hashed
    /// using keccak256.
    pub fn hash_message<S>(&self, message: S) -> H256
    where
        S: AsRef<[u8]>,
    {
        let message = message.as_ref();

        let mut eth_message = format!("\x19Ethereum Signed Message:\n{}", message.len()).into_bytes();
        eth_message.extend_from_slice(message);

        signing::keccak256(&eth_message).into()
    }

    /// Sign arbitrary string data.
    ///
    /// The data is UTF-8 encoded and enveloped the same way as with
    /// `hash_message`. The returned signed data's signature is in 'Electrum'
    /// notation, that is the recovery value `v` is either `27` or `28` (as
    /// opposed to the standard notation where `v` is either `0` or `1`). This
    /// is important to consider when using this signature with other crates.
    pub fn sign<S>(&self, message: S, key: impl signing::Key) -> SignedData
    where
        S: AsRef<[u8]>,
    {
        let message = message.as_ref();
        let message_hash = self.hash_message(message);

        let signature = key
            .sign(&message_hash.as_bytes(), None)
            .expect("hash is non-zero 32-bytes; qed");
        let v = signature
            .v
            .try_into()
            .expect("signature recovery in electrum notation always fits in a u8");

        let signature_bytes = Bytes({
            let mut bytes = Vec::with_capacity(65);
            bytes.extend_from_slice(signature.r.as_bytes());
            bytes.extend_from_slice(signature.s.as_bytes());
            bytes.push(v);
            bytes
        });

        // We perform this allocation only after all previous fallible actions have completed successfully.
        let message = message.to_owned();

        SignedData {
            message,
            message_hash,
            v,
            r: signature.r,
            s: signature.s,
            signature: signature_bytes,
        }
    }

    /// Recovers the Ethereum address which was used to sign the given data.
    ///
    /// Recovery signature data uses 'Electrum' notation, this means the `v`
    /// value is expected to be either `27` or `28`.
    pub fn recover<R>(&self, recovery: R) -> error::Result<Address>
    where
        R: Into<Recovery>,
    {
        let recovery = recovery.into();
        let message_hash = match recovery.message {
            RecoveryMessage::Data(ref message) => self.hash_message(message),
            RecoveryMessage::Hash(hash) => hash,
        };
        let (signature, recovery_id) = recovery
            .as_signature()
            .ok_or_else(|| error::Error::Recovery(signing::RecoveryError::InvalidSignature))?;
        let address = signing::recover(message_hash.as_bytes(), &signature, recovery_id)?;
        Ok(address)
    }
}

type MaybeReady<T, R> = Either<future::Ready<error::Result<R>>, CallFuture<R, <T as Transport>::Out>>;

type TxParams<T> = Join3<MaybeReady<T, U256>, MaybeReady<T, U256>, MaybeReady<T, U256>>;

/// Future resolving when transaction signing is complete.
///
/// Transaction signing can perform RPC requests in order to fill missing
/// parameters required for signing `nonce`, `gas_price` and `chain_id`. Note
/// that if all transaction parameters were provided, this future will resolve
/// immediately.
pub struct SignTransactionFuture<T: Transport, K> {
    tx: TransactionParameters,
    key: Option<K>,
    inner: TxParams<T>,
}

impl<T: Transport, K: signing::Key> SignTransactionFuture<T, K> {
    /// Creates a new SignTransactionFuture with accounts and transaction data.
    pub fn new(accounts: &Accounts<T>, tx: TransactionParameters, key: K) -> SignTransactionFuture<T, K> {
        macro_rules! maybe {
            ($o: expr, $f: expr) => {
                match $o {
                    Some(ref value) => Either::Left(future::ok(value.clone())),
                    None => Either::Right($f),
                }
            };
        }

        let from = key.address();
        let inner = future::join3(
            maybe!(tx.nonce, accounts.web3().eth().transaction_count(from, None)),
            maybe!(tx.gas_price, accounts.web3().eth().gas_price()),
            maybe!(tx.chain_id.map(U256::from), accounts.web3().eth().chain_id()),
        );

        SignTransactionFuture {
            tx,
            key: Some(key),
            inner,
        }
    }
}

impl<T: Transport, K: signing::Key> Future for SignTransactionFuture<T, K> {
    type Output = error::Result<SignedTransaction>;

    fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Self::Output> {
        let (nonce, gas_price, chain_id) = ready!(self.inner.poll_unpin(ctx));
        let chain_id = chain_id?.as_u64();

        let data = mem::replace(&mut self.tx.data, Bytes::default());
        let tx = Transaction {
            to: self.tx.to,
            nonce: nonce?,
            gas: self.tx.gas,
            gas_price: gas_price?,
            value: self.tx.value,
            data: data.0,
        };
        let signed = tx.sign(
            self.key
                .take()
                .expect("SignTransactionFuture can't be polled after Ready; qed"),
            chain_id,
        );

        Poll::Ready(Ok(signed))
    }
}

/// A transaction used for RLP encoding, hashing and signing.
struct Transaction {
    to: Option<Address>,
    nonce: U256,
    gas: U256,
    gas_price: U256,
    value: U256,
    data: Vec<u8>,
}

impl Transaction {
    /// RLP encode an unsigned transaction for the specified chain ID.
    fn rlp_append_unsigned(&self, rlp: &mut RlpStream, chain_id: u64) {
        rlp.begin_list(9);
        rlp.append(&self.nonce);
        rlp.append(&self.gas_price);
        rlp.append(&self.gas);
        if let Some(to) = self.to {
            rlp.append(&to);
        } else {
            rlp.append(&"");
        }
        rlp.append(&self.value);
        rlp.append(&self.data);
        rlp.append(&chain_id);
        rlp.append(&0u8);
        rlp.append(&0u8);
    }

    /// RLP encode a signed transaction with the specified signature.
    fn rlp_append_signed(&self, rlp: &mut RlpStream, signature: &Signature) {
        rlp.begin_list(9);
        rlp.append(&self.nonce);
        rlp.append(&self.gas_price);
        rlp.append(&self.gas);
        if let Some(to) = self.to {
            rlp.append(&to);
        } else {
            rlp.append(&"");
        }
        rlp.append(&self.value);
        rlp.append(&self.data);
        rlp.append(&signature.v);
        rlp.append(&U256::from_big_endian(signature.r.as_bytes()));
        rlp.append(&U256::from_big_endian(signature.s.as_bytes()));
    }

    /// Sign and return a raw signed transaction.
    fn sign(self, sign: impl signing::Key, chain_id: u64) -> SignedTransaction {
        let mut rlp = RlpStream::new();
        self.rlp_append_unsigned(&mut rlp, chain_id);

        let hash = signing::keccak256(rlp.as_raw());
        let signature = sign
            .sign(&hash, Some(chain_id))
            .expect("hash is non-zero 32-bytes; qed");

        rlp.clear();
        self.rlp_append_signed(&mut rlp, &signature);

        let transaction_hash = signing::keccak256(rlp.as_raw()).into();
        let raw_transaction = rlp.out().into();

        SignedTransaction {
            message_hash: hash.into(),
            v: signature.v,
            r: signature.r,
            s: signature.s,
            raw_transaction,
            transaction_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tests::TestTransport;
    use crate::signing::{SecretKey, SecretKeyRef};
    use crate::types::Bytes;
    use rustc_hex::FromHex;
    use serde_json::json;

    #[test]
    fn accounts_sign_transaction() {
        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.0/web3-eth-accounts.html#eth-accounts-signtransaction

        let tx = TransactionParameters {
            to: Some("F0109fC8DF283027b6285cc889F5aA624EaC1F55".parse().unwrap()),
            value: 1_000_000_000.into(),
            gas: 2_000_000.into(),
            ..Default::default()
        };
        let key: SecretKey = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();
        let nonce = U256::zero();
        let gas_price = U256::from(21_000_000_000u128);
        let chain_id = "0x1";
        let from: Address = signing::secret_key_address(&key);

        let mut transport = TestTransport::default();
        transport.add_response(json!(nonce));
        transport.add_response(json!(gas_price));
        transport.add_response(json!(chain_id));

        let signed = {
            let accounts = Accounts::new(&transport);
            futures::executor::block_on(accounts.sign_transaction(tx, &key))
        };

        transport.assert_request(
            "eth_getTransactionCount",
            &[json!(from).to_string(), json!("latest").to_string()],
        );
        transport.assert_request("eth_gasPrice", &[]);
        transport.assert_request("eth_chainId", &[]);
        transport.assert_no_more_requests();

        let expected = SignedTransaction {
            message_hash: "88cfbd7e51c7a40540b233cf68b62ad1df3e92462f1c6018d6d67eae0f3b08f5"
                .parse()
                .unwrap(),
            v: 0x25,
            r: "c9cf86333bcb065d140032ecaab5d9281bde80f21b9687b3e94161de42d51895"
                .parse()
                .unwrap(),
            s: "727a108a0b8d101465414033c3f705a9c7b826e596766046ee1183dbc8aeaa68"
                .parse()
                .unwrap(),
            raw_transaction: Bytes(
                "f869808504e3b29200831e848094f0109fc8df283027b6285cc889f5aa624eac1f55843b9aca008025a0c9cf86333bcb065d140032ecaab5d9281bde80f21b9687b3e94161de42d51895a0727a108a0b8d101465414033c3f705a9c7b826e596766046ee1183dbc8aeaa68"
                    .from_hex()
                    .unwrap(),
            ),
            transaction_hash: "de8db924885b0803d2edc335f745b2b8750c8848744905684c20b987443a9593"
                .parse()
                .unwrap(),
        };

        assert_eq!(signed, Ok(expected));
    }

    #[test]
    fn accounts_sign_transaction_with_all_parameters() {
        let key: SecretKey = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
            .parse()
            .unwrap();

        let accounts = Accounts::new(TestTransport::default());
        futures::executor::block_on(accounts.sign_transaction(
            TransactionParameters {
                nonce: Some(0.into()),
                gas_price: Some(1.into()),
                chain_id: Some(42),
                ..Default::default()
            },
            &key,
        ))
        .unwrap();

        // sign_transaction makes no requests when all parameters are specified
        accounts.transport().assert_no_more_requests();
    }

    #[test]
    fn accounts_hash_message() {
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

        // this method does not actually make any requests.
        accounts.transport().assert_no_more_requests();
    }

    #[test]
    fn accounts_sign() {
        // test vector taken from:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#sign

        let accounts = Accounts::new(TestTransport::default());

        let key: SecretKey = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();
        let signed = accounts.sign("Some data", SecretKeyRef::new(&key));

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

        // this method does not actually make any requests.
        accounts.transport().assert_no_more_requests();
    }

    #[test]
    fn accounts_recover() {
        // test vector taken from:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#recover

        let accounts = Accounts::new(TestTransport::default());

        let v = 0x1cu64;
        let r: H256 = "b91467e570a6466aa9e9876cbcd013baba02900b8979d43fe208a4a4f339f5fd"
            .parse()
            .unwrap();
        let s: H256 = "6007e74cd82e037b800186422fc2da167c747ef045e5d18a5f5d4300f8e1a029"
            .parse()
            .unwrap();

        let recovery = Recovery::new("Some data", v, r, s);
        assert_eq!(
            accounts.recover(recovery).unwrap(),
            "2c7536E3605D9C16a7a3D7b1898e529396a65c23".parse().unwrap()
        );

        // this method does not actually make any requests.
        accounts.transport().assert_no_more_requests();
    }

    #[test]
    fn accounts_recover_signed() {
        let key: SecretKey = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
            .parse()
            .unwrap();
        let address: Address = signing::secret_key_address(&key);

        let accounts = Accounts::new(TestTransport::default());

        let signed = accounts.sign("rust-web3 rocks!", &key);
        let recovered = accounts.recover(&signed).unwrap();
        assert_eq!(recovered, address);

        let signed = futures::executor::block_on(accounts.sign_transaction(
            TransactionParameters {
                nonce: Some(0.into()),
                gas_price: Some(1.into()),
                chain_id: Some(42),
                ..Default::default()
            },
            &key,
        ))
        .unwrap();
        let recovered = accounts.recover(&signed).unwrap();
        assert_eq!(recovered, address);

        // these methods make no requests
        accounts.transport().assert_no_more_requests();
    }

    #[test]
    fn sign_transaction_data() {
        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#eth-accounts-signtransaction

        let tx = Transaction {
            nonce: 0.into(),
            gas: 2_000_000.into(),
            gas_price: 234_567_897_654_321u64.into(),
            to: Some("F0109fC8DF283027b6285cc889F5aA624EaC1F55".parse().unwrap()),
            value: 1_000_000_000.into(),
            data: Vec::new(),
        };
        let skey: SecretKey = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();
        let key = SecretKeyRef::new(&skey);

        let signed = tx.sign(key, 1);

        let expected = SignedTransaction {
            message_hash: "6893a6ee8df79b0f5d64a180cd1ef35d030f3e296a5361cf04d02ce720d32ec5"
                .parse()
                .unwrap(),
            v: 0x25,
            r: "09ebb6ca057a0535d6186462bc0b465b561c94a295bdb0621fc19208ab149a9c"
                .parse()
                .unwrap(),
            s: "440ffd775ce91a833ab410777204d5341a6f9fa91216a6f3ee2c051fea6a0428"
                .parse()
                .unwrap(),
            raw_transaction: Bytes(
                "f86a8086d55698372431831e848094f0109fc8df283027b6285cc889f5aa624eac1f55843b9aca008025a009ebb6ca057a0535d6186462bc0b465b561c94a295bdb0621fc19208ab149a9ca0440ffd775ce91a833ab410777204d5341a6f9fa91216a6f3ee2c051fea6a0428"
                    .from_hex()
                    .unwrap(),
            ),
            transaction_hash: "d8f64a42b57be0d565f385378db2f6bf324ce14a594afc05de90436e9ce01f60"
                .parse()
                .unwrap(),
        };

        assert_eq!(signed, expected);
    }
}
