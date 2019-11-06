//! Partial implementation of the `Accounts` namespace.

use crate::api::{Namespace, Web3};
use crate::error::Error;
use crate::helpers::CallFuture;
use crate::types::{Address, Bytes, SignedData, SignedTransaction, TransactionParameters, H256, U256};
use crate::Transport;
use ethsign::{self, SecretKey, Signature};
use futures::future::{self, Either, FutureResult, Join3};
use futures::{Future, Poll};
use parity_crypto::Keccak256;
use rlp::RlpStream;

/// Transaction base fee, this will be used as the default transaction fee in
/// the case none is specified for signing.
pub const TRANSACTION_BASE_GAS_FEE: U256 = U256([21000, 0, 0, 0]);

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
    pub fn sign_transaction(&self, tx: TransactionParameters, key: SecretKey) -> SignTransactionFuture<T> {
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

type TxParams<T> = Join3<MaybeReady<T, U256>, MaybeReady<T, U256>, MaybeReady<T, String>>;

/// Future resolving when transaction signing is complete
pub struct SignTransactionFuture<T: Transport> {
    accounts: Accounts<T>,
    tx: TransactionParameters,
    key: SecretKey,
    inner: TxParams<T>,
}

impl<T: Transport> SignTransactionFuture<T> {
    /// Creates a new SignTransactionFuture with accounts and transaction data.
    pub fn new(accounts: Accounts<T>, tx: TransactionParameters, key: SecretKey) -> SignTransactionFuture<T> {
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
        let tx = RawTxParameters {
            nonce,
            to: self.tx.to.unwrap_or_default(),
            value: self.tx.value.unwrap_or_default(),
            gas_price,
            gas: self.tx.gas.unwrap_or(TRANSACTION_BASE_GAS_FEE),
            data: self.tx.data.take().unwrap_or_default(),
        };

        unimplemented!();
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

/// Raw transaction data for signing. When a transaction is signed, all
/// parameter values need to be finalized for signing, and all parameters are
/// required. Note that transaction data does not actually have the `from`
/// public address as that is recoverable from the signature.
#[derive(Debug)]
struct RawTxParameters {
    /// Transaction nonce (None for account transaction count)
    pub nonce: U256,
    /// To address
    pub to: Address,
    /// Supplied gas (None for sensible default)
    pub gas: U256,
    /// Gas price (None for sensible default)
    pub gas_price: U256,
    /// Transfered value (None for no transfer)
    pub value: U256,
    /// Data (None for empty data)
    pub data: Bytes,
}

impl RawTxParameters {
    /// Sign and return a raw transaction.
    fn sign(&self, key: &SecretKey, chain_id: Option<u64>) -> Result<Bytes, ethsign::Error> {
        let mut rlp = RlpStream::new();
        self.rlp_append_unsigned(&mut rlp, chain_id);
        let hash = rlp.as_raw().keccak256();
        rlp.clear();

        let sig = key.sign(&hash[..])?;
        self.rlp_append_signed(&mut rlp, sig, chain_id);

        Ok(rlp.out().into())
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
    fn rlp_append_signed(&self, s: &mut RlpStream, sig: Signature, chain_id: Option<u64>) {
        let v = add_chain_replay_protection(u64::from(sig.v), chain_id);

        s.begin_list(9);
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.to);
        s.append(&self.value);
        s.append(&self.data.0);
        s.append(&v);
        s.append(&U256::from(sig.r));
        s.append(&U256::from(sig.s));
    }
}

/// Encode chain ID based on
/// (EIP-155)[https://github.com/ethereum/EIPs/blob/master/EIPS/eip-155.md)
fn add_chain_replay_protection(v: u64, chain_id: Option<u64>) -> u64 {
    v + if let Some(n) = chain_id { 35 + n * 2 } else { 27 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::helpers::tests::TestTransport;
    use crate::types::{Bytes, SignedData};
    use ethsign::{SecretKey, Signature};
    use rustc_hex::FromHex;

    #[test]
    fn tx_base_gas_fee() {
        // make sure the endianness is right for the TX_BASE_FEE constant

        assert_eq!(TRANSACTION_BASE_GAS_FEE, 21000.into());
    }

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
    fn sign_transaction() {}

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
                .from_hex()
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

    #[test]
    fn raw_tx_params_sign() {
        // retrieved test vector from:
        // https://web3js.readthedocs.io/en/v1.2.0/web3-eth-accounts.html#eth-accounts-signtransaction

        let tx = RawTxParameters {
            nonce: 0.into(),
            gas: 2_000_000.into(),
            gas_price: 234_567_897_654_321u64.into(),
            to: "F0109fC8DF283027b6285cc889F5aA624EaC1F55".parse().unwrap(),
            value: 1_000_000_000.into(),
            data: Bytes::default(),
        };
        let key: H256 = "4c0883a69102937d6231471b5dbb6204fe5129617082792ae468d01a3f362318"
            .parse()
            .unwrap();
        let raw = tx
            .sign(&SecretKey::from_raw(&key[..]).expect("valid key"), Some(1))
            .unwrap();

        let expected = Bytes(
            "f86a8086d55698372431831e848094f0109fc8df283027b6285cc889f5aa624eac1f55843b9aca008025a009ebb6ca057a0535d6186462bc0b465b561c94a295bdb0621fc19208ab149a9ca0440ffd775ce91a833ab410777204d5341a6f9fa91216a6f3ee2c051fea6a0428"
                .from_hex()
                .unwrap(),
        );

        assert_eq!(raw, expected);
    }
}
