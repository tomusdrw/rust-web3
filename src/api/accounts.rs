//! Partial implementation of the `Accounts` namespace.

use crate::api::{Namespace, Web3};
use crate::error::Error;
use crate::helpers::CallFuture;
use crate::types::{
    Address, Bytes, Recovery, RecoveryMessage, SignedData, SignedTransaction, TransactionParameters, H256, U256,
};
use crate::Transport;
use futures::future::{self, Either, FutureResult, Join3};
use futures::{Async, Future, Poll};
use rlp::RlpStream;
use secp256k1::key::ONE_KEY;
use secp256k1::{Message, PublicKey, Secp256k1, SecretKey};
use std::convert::TryInto;
use std::mem;
use std::ops::Deref;
use tiny_keccak::{Hasher, Keccak};
use zeroize::{DefaultIsZeroes, Zeroize};

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
    pub fn sign_transaction(&self, tx: TransactionParameters, key: &SecretKey) -> SignTransactionFuture<T> {
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

        keccak256(&eth_message).into()
    }

    /// Sign arbitrary string data.
    ///
    /// The data is UTF-8 encoded and enveloped the same way as with
    /// `hash_message`. The returned signed data's signature is in 'Electrum'
    /// notation, that is the recovery value `v` is either `27` or `28` (as
    /// opposed to the standard notation where `v` is either `0` or `1`). This
    /// is important to consider when using this signature with other crates.
    pub fn sign<S>(&self, message: S, key: &SecretKey) -> SignedData
    where
        S: AsRef<[u8]>,
    {
        let message = message.as_ref();
        let message_hash = self.hash_message(message);

        let sig_message = Message::from_slice(message_hash.as_bytes()).expect("hash is non-zero 32-bytes; qed");
        let signature = sign(&sig_message, key, None);
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
    pub fn recover<R>(&self, recovery: R) -> Result<Address, Error>
    where
        R: Into<Recovery>,
    {
        let recovery = recovery.into();
        let message_hash = match recovery.message {
            RecoveryMessage::Data(ref message) => self.hash_message(message),
            RecoveryMessage::Hash(hash) => hash,
        };
        let signature = recovery.as_signature()?;

        let message = Message::from_slice(message_hash.as_bytes())?;
        let public_key = Secp256k1::verification_only().recover(&message, &signature)?;

        Ok(public_key_address(&public_key))
    }
}

/// Compute the Keccak-256 hash of input bytes.
pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    hasher.finalize(&mut output);
    output
}

/// Gets the public address of a private key.
fn secret_key_address(key: &SecretKey) -> Address {
    let secp = Secp256k1::signing_only();
    let public_key = PublicKey::from_secret_key(&secp, key);
    public_key_address(&public_key)
}

/// Gets the address of a public key.
///
/// The public address is defined as the low 20 bytes of the keccak hash of
/// the public key. Note that the public key returned from the `secp256k1`
/// crate is 65 bytes long, that is because it is prefixed by `0x04` to
/// indicate an uncompressed public key; this first byte is ignored when
/// computing the hash.
fn public_key_address(public_key: &PublicKey) -> Address {
    let public_key = public_key.serialize_uncompressed();

    debug_assert_eq!(public_key[0], 0x04);
    let hash = keccak256(&public_key[1..]);

    Address::from_slice(&hash[12..])
}

type MaybeReady<T, R> = Either<FutureResult<R, Error>, CallFuture<R, <T as Transport>::Out>>;

type TxParams<T> = Join3<MaybeReady<T, U256>, MaybeReady<T, U256>, MaybeReady<T, U256>>;

/// Future resolving when transaction signing is complete.
///
/// Transaction signing can perform RPC requests in order to fill missing
/// parameters required for signing `nonce`, `gas_price` and `chain_id`. Note
/// that if all transaction parameters were provided, this future will resolve
/// immediately.
pub struct SignTransactionFuture<T: Transport> {
    tx: TransactionParameters,
    key: ZeroizeSecretKey,
    inner: TxParams<T>,
}

impl<T: Transport> SignTransactionFuture<T> {
    /// Creates a new SignTransactionFuture with accounts and transaction data.
    pub fn new(accounts: &Accounts<T>, tx: TransactionParameters, key: &SecretKey) -> SignTransactionFuture<T> {
        macro_rules! maybe {
            ($o: expr, $f: expr) => {
                match $o.clone() {
                    Some(value) => Either::A(future::ok(value)),
                    None => Either::B($f),
                }
            };
        }

        let from = secret_key_address(key);
        let inner = Future::join3(
            maybe!(tx.nonce, accounts.web3().eth().transaction_count(from, None)),
            maybe!(tx.gas_price, accounts.web3().eth().gas_price()),
            maybe!(tx.chain_id.map(U256::from), accounts.web3().eth().chain_id()),
        );

        SignTransactionFuture {
            tx,
            key: ZeroizeSecretKey(*key),
            inner,
        }
    }
}

impl<T: Transport> Future for SignTransactionFuture<T> {
    type Item = SignedTransaction;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (nonce, gas_price, chain_id) = try_ready!(self.inner.poll());
        let chain_id = chain_id.as_u64();

        let data = mem::replace(&mut self.tx.data, Bytes::default());
        let tx = Transaction {
            to: self.tx.to,
            nonce,
            gas: self.tx.gas,
            gas_price,
            value: self.tx.value,
            data: data.0,
        };
        let signed = tx.sign(&self.key, chain_id);

        Ok(Async::Ready(signed))
    }
}

impl<T: Transport> Drop for SignTransactionFuture<T> {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

/// A struct that represents a the components of a secp256k1 signature.
struct Signature {
    v: u64,
    r: H256,
    s: H256,
}

/// Sign a message with a secret key and optional chain ID.
///
/// When a chain ID is provided, the `Signature`'s V-value will have chain relay
/// protection added (as per EIP-155). Otherwise, the V-value will be in
/// 'Electrum' notation.
fn sign(message: &Message, key: &SecretKey, chain_id: Option<u64>) -> Signature {
    let (recovery_id, signature) = Secp256k1::signing_only()
        .sign_recoverable(message, key)
        .serialize_compact();

    let standard_v = recovery_id.to_i32() as u64;
    let v = if let Some(chain_id) = chain_id {
        // When signing with a chain ID, add chain replay protection.
        standard_v + 35 + chain_id * 2
    } else {
        // Otherwise, convert to 'Electrum' notation.
        standard_v + 27
    };
    let r = H256::from_slice(&signature[..32]);
    let s = H256::from_slice(&signature[32..]);

    Signature { v, r, s }
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
    fn sign(self, key: &SecretKey, chain_id: u64) -> SignedTransaction {
        let mut rlp = RlpStream::new();
        self.rlp_append_unsigned(&mut rlp, chain_id);

        let hash = keccak256(rlp.as_raw());
        let message = Message::from_slice(&hash).expect("hash is non-zero 32-bytes; qed");
        let signature = sign(&message, key, Some(chain_id));

        rlp.clear();
        self.rlp_append_signed(&mut rlp, &signature);

        let transaction_hash = keccak256(rlp.as_raw()).into();
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

/// A wrapper type around `SecretKey` to prevent leaking secret key data. This
/// type will properly zeroize the secret key to `ONE_KEY` in a way that will
/// not get optimized away by the compiler nor be prone to leaks that take
/// advantage of access reordering.
///
/// This is required since the `SignTransactionFuture` needs to retain a copy
/// of the `SecretKey`.
#[derive(Clone, Copy)]
struct ZeroizeSecretKey(SecretKey);

impl Default for ZeroizeSecretKey {
    fn default() -> Self {
        ZeroizeSecretKey(ONE_KEY)
    }
}

impl Deref for ZeroizeSecretKey {
    type Target = SecretKey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DefaultIsZeroes for ZeroizeSecretKey {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tests::TestTransport;
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
        let from: Address = secret_key_address(&key);

        let mut transport = TestTransport::default();
        transport.add_response(json!(nonce));
        transport.add_response(json!(gas_price));
        transport.add_response(json!(chain_id));

        let signed = {
            let accounts = Accounts::new(&transport);
            accounts.sign_transaction(tx, &key).wait()
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
        accounts
            .sign_transaction(
                TransactionParameters {
                    nonce: Some(0.into()),
                    gas_price: Some(1.into()),
                    chain_id: Some(42),
                    ..Default::default()
                },
                &key,
            )
            .wait()
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
        let signed = accounts.sign("Some data", &key);

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
        let address: Address = secret_key_address(&key);

        let accounts = Accounts::new(TestTransport::default());

        let signed = accounts.sign("rust-web3 rocks!", &key);
        let recovered = accounts.recover(&signed).unwrap();
        assert_eq!(recovered, address);

        let signed = accounts
            .sign_transaction(
                TransactionParameters {
                    nonce: Some(0.into()),
                    gas_price: Some(1.into()),
                    chain_id: Some(42),
                    ..Default::default()
                },
                &key,
            )
            .wait()
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
        let key: SecretKey = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();

        let signed = tx.sign(&key, 1);

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
