//! Partial implementation of the `Accounts` namespace.

use crate::api::{Namespace, Web3};
use crate::error::Error;
use crate::helpers::CallFuture;
use crate::types::{Address, Bytes, SignedData, SignedTransaction, TransactionParameters, H256, U256};
use crate::Transport;
use ethereum_transaction::{self as ethtx, Transaction};
use ethsign::{Error as EthsignError, SecretKey, Signature};
use futures::future::{self, Either, FutureResult, Join3};
use futures::{Async, Future, Poll};
use parity_crypto::Keccak256;
use std::borrow::Cow;
use std::mem;

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
    /// The data will be UTF-8 HEX decoded and enveloped as
    /// follows: `"\x19Ethereum Signed Message:\n" + message.length + message`
    /// and hashed using keccak256.
    pub fn hash_message<S>(&self, message: S) -> H256
    where
        S: AsRef<str>,
    {
        let message = message.as_ref();
        let eth_message = format!("\x19Ethereum Signed Message:\n{}{}", message.len(), message);

        eth_message.as_bytes().keccak256().into()
    }

    /// Sign arbitrary string data.
    ///
    /// The data is UTF-8 encoded and enveloped the same way as with
    /// `hash_message`.
    pub fn sign<S>(&self, message: S, key: &SecretKey) -> Result<SignedData, Error>
    where
        S: AsRef<str>,
    {
        let message = message.as_ref().to_string();
        let message_hash = self.hash_message(&message);

        let signature = key.sign(&message_hash[..]).map_err(EthsignError::from)?;
        // We convert the signature to 'Electrum' notation. Usually signatures'
        // recovery value `v` is either `0` or `1` but under this notation the
        // recovery value is either `27` or `28`.
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
    pub fn recover<S, Sig>(&self, message: S, signature: Sig) -> Result<Address, Error>
    where
        S: AsRef<str>,
        Sig: IntoSignature,
    {
        let message_hash = self.hash_message(message);
        let signature = signature.into_signature();

        let public_key = signature.recover(&message_hash[..]).map_err(EthsignError::from)?;

        Ok(public_key.address().into())
    }
}

type MaybeReady<T, R> = Either<FutureResult<R, Error>, CallFuture<R, <T as Transport>::Out>>;

type TxParams<T> = Join3<MaybeReady<T, U256>, MaybeReady<T, U256>, MaybeReady<T, String>>;

/// Future resolving when transaction signing is complete.
///
/// Transaction signing can perform RPC requests in order to fill missing
/// parameters required for signing `nonce`, `gas_price` and `chain_id`. Note
/// that if all transaction parameters were provided, this future will resolve
/// immediately.
pub struct SignTransactionFuture<T: Transport> {
    tx: TransactionParameters,
    key: SecretKey,
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

        let from = key.public().address().into();
        let inner = Future::join3(
            maybe!(tx.nonce, accounts.web3().eth().transaction_count(from, None)),
            maybe!(tx.gas_price, accounts.web3().eth().gas_price()),
            maybe!(None, accounts.web3().net().version()),
        );

        SignTransactionFuture {
            tx,
            key: key.clone(),
            inner,
        }
    }
}

impl<T: Transport> Future for SignTransactionFuture<T> {
    type Item = SignedTransaction;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let (nonce, gas_price, chain_id) = try_ready!(self.inner.poll());
        let chain_id = chain_id.parse::<u64>().map_err(|e| Error::Decoder(e.to_string()))?;

        // we need to convert `Address` and `U256` between two versions of
        // `ethereum-types`; since they have identical memory layouts we can
        // safely transmute them
        macro_rules! t {
            ($value:expr) => {
                unsafe { mem::transmute($value) }
            };
        }

        let data = mem::replace(&mut self.tx.data, Bytes::default());
        let tx = Transaction {
            from: t!(Address::zero()), // not used for signing.
            to: self.tx.to.map(|to| t!(to)),
            nonce: t!(nonce),
            gas: t!(self.tx.gas),
            gas_price: t!(gas_price),
            value: t!(self.tx.value),
            data: ethtx::Bytes(data.0),
        };
        let signed = sign_transaction(tx, &self.key, chain_id)?;

        Ok(Async::Ready(signed))
    }
}

/// Trait for converting data into an `ethsign::Signature`.
pub trait IntoSignature {
    fn into_signature(self) -> Signature;
}

impl IntoSignature for (u8, H256, H256) {
    fn into_signature(self) -> Signature {
        let (v, r, s) = self;
        Signature {
            v: v - 27, // Convert from 'Electrum' notation
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

impl IntoSignature for Signature {
    fn into_signature(self) -> Signature {
        self
    }
}

/// Sign and return a raw signed transaction.
fn sign_transaction(tx: Transaction, key: &SecretKey, chain_id: u64) -> Result<SignedTransaction, EthsignError> {
    let tx = ethtx::SignTransaction {
        transaction: Cow::Owned(tx),
        chain_id,
    };

    let hash = tx.hash();
    let sig = key.sign(&hash[..])?;

    let signed_tx = ethtx::SignedTransaction::new(tx.transaction, tx.chain_id, sig.v, sig.r, sig.s);
    let transaction_hash = signed_tx.hash().into();
    let raw_transaction = Bytes(signed_tx.to_rlp());

    Ok(SignedTransaction {
        message_hash: hash.into(),
        v: signed_tx.v,
        r: sig.r.into(),
        s: sig.s.into(),
        raw_transaction,
        transaction_hash,
    })
}

/*
 * TODO(nlordell): remove this?

/// Raw transaction data for signing. When a transaction is signed, all
/// parameter values need to be finalized for signing, and all parameters are
/// required. Note that transaction data does not actually have the `from`
/// public address as that is recoverable from the signature.
#[derive(Debug)]
struct RawTxParameters {
    /// Transaction nonce (None for account transaction count)
    nonce: U256,
    /// To address
    to: Address,
    /// Supplied gas (None for sensible default)
    gas: U256,
    /// Gas price (None for sensible default)
    gas_price: U256,
    /// Transfered value (None for no transfer)
    value: U256,
    /// Data (None for empty data)
    data: Bytes,
}

impl RawTxParameters {
    /// Sign and return a raw transaction.
    fn sign(&self, key: &SecretKey, chain_id: Option<u64>) -> Result<SignedTransaction, EthsignError> {
        let mut rlp = RlpStream::new();

        self.rlp_append_unsigned(&mut rlp, chain_id);
        let hash = rlp.as_raw().keccak256();
        let sig = key.sign(&hash[..])?;

        rlp.clear();
        self.rlp_append_signed(&mut rlp, &sig, chain_id);
        let transaction_hash = rlp.as_raw().keccak256().into();

        Ok(SignedTransaction {
            message_hash: hash.into(),
            v: add_chain_replay_protection(sig.v, chain_id),
            r: sig.r.into(),
            s: sig.s.into(),
            raw_transaction: rlp.out().into(),
            transaction_hash,
        })
    }

    /// RLP encode an unsigned transaction.
    fn rlp_append_unsigned(&self, s: &mut RlpStream, chain_id: Option<u64>) {
        s.begin_list(if chain_id.is_some() { 9 } else { 6 });
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.to);
        s.append(&self.value);
        s.append(&self.data.0);
        if let Some(n) = chain_id {
            s.append(&n);
            s.append(&0u8);
            s.append(&0u8);
        }
    }

    /// RLP encode a transaction with its signature.
    fn rlp_append_signed(&self, s: &mut RlpStream, sig: &Signature, chain_id: Option<u64>) {
        s.begin_list(9);
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.to);
        s.append(&self.value);
        s.append(&self.data.0);
        s.append(&add_chain_replay_protection(sig.v, chain_id));
        s.append(&U256::from(sig.r));
        s.append(&U256::from(sig.s));
    }
}

/// Encode chain ID based on
/// (EIP-155)[https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md)
fn add_chain_replay_protection(v: u8, chain_id: Option<u64>) -> u64 {
    let v = v as u64;
    v + if let Some(n) = chain_id { 35 + n * 2 } else { 27 }
}

 */

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tests::TestTransport;
    use crate::types::{Bytes, SignedData};
    use ethsign::{SecretKey, Signature};
    use rustc_hex::FromHex;
    use serde_json::json;

    #[test]
    fn accounts_sign_transaction() {
        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.0/web3-eth-accounts.html#eth-accounts-signtransaction

        let tx = TransactionParameters {
            to: Some("F0109fC8DF283027b6285cc889F5aA624EaC1F55".parse().unwrap()),
            value: 1000000000.into(),
            gas: 2000000.into(),
            ..Default::default()
        };
        let secret: H256 = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();
        let key = SecretKey::from_raw(&secret[..]).unwrap();
        let nonce = U256::zero();
        let gas_price = U256::from(21000000000u128);
        let chain_id = "1";
        let from: Address = key.public().address().into();

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
        transport.assert_request("net_version", &[]);
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

        // this method does not actually make any requests.
        accounts.transport().assert_no_more_requests();
    }

    #[test]
    fn accounts_recover() {
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

        // this method does not actually make any requests.
        accounts.transport().assert_no_more_requests();
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
            message_hash: "1da44b586eb0729ff70a73c326926f6ed5a25f5b056e7f47fbc6e58d86871655"
                .parse()
                .unwrap(),
            v,
            r,
            s,
            signature: Bytes(
                "b91467e570a6466aa9e9876cbcd013baba02900b8979d43fe208a4a4f339f5fd6007e74cd82e037b800186422fc2da167c747ef045e5d18a5f5d4300f8e1a0291c"
                    .from_hex()
                    .unwrap()
            ),
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

    #[test]
    fn sign_ethtx_transaction() {
        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.2/web3-eth-accounts.html#eth-accounts-signtransaction

        let tx = Transaction {
            from: Default::default(), // not used for signing
            nonce: 0.into(),
            gas: 2_000_000.into(),
            gas_price: 234_567_897_654_321u64.into(),
            to: Some("F0109fC8DF283027b6285cc889F5aA624EaC1F55".parse().unwrap()),
            value: 1_000_000_000.into(),
            data: ethtx::Bytes(Vec::new()),
        };
        let key = {
            let raw: H256 = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
                .parse()
                .unwrap();
            SecretKey::from_raw(&raw[..]).expect("valid key")
        };

        let signed = sign_transaction(tx, &key, 1).unwrap();

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
