//! Signing capabilities and utilities.

use crate::types::H256;

/// Error during signing.
#[derive(Debug, derive_more::Display, PartialEq, Clone)]
pub enum SigningError {
    /// A message to sign is invalid. Has to be a non-zero 32-bytes slice.
    #[display(fmt = "Message has to be a non-zero 32-bytes slice.")]
    InvalidMessage,
}
impl std::error::Error for SigningError {}

/// Error during sender recovery.
#[derive(Debug, derive_more::Display, PartialEq, Clone)]
pub enum RecoveryError {
    /// A message to recover is invalid. Has to be a non-zero 32-bytes slice.
    #[display(fmt = "Message has to be a non-zero 32-bytes slice.")]
    InvalidMessage,
    /// A signature is invalid and the sender could not be recovered.
    #[display(fmt = "Signature is invalid (check recovery id).")]
    InvalidSignature,
}
impl std::error::Error for RecoveryError {}

#[cfg(feature = "signing")]
pub use feature_gated::*;

#[cfg(feature = "signing")]
mod feature_gated {
    use super::*;
    use crate::types::Address;
    use once_cell::sync::Lazy;
    pub(crate) use secp256k1::SecretKey;
    use secp256k1::{
        recovery::{RecoverableSignature, RecoveryId},
        All, Message, PublicKey, Secp256k1,
    };
    use std::ops::Deref;

    static CONTEXT: Lazy<Secp256k1<All>> = Lazy::new(Secp256k1::new);

    /// A trait representing ethereum-compatible key with signing capabilities.
    ///
    /// The purpose of this trait is to prevent leaking `secp256k1::SecretKey` struct
    /// in stack or memory.
    /// To use secret keys securely, they should be wrapped in a struct that prevents
    /// leaving copies in memory (both when it's moved or dropped). Please take a look
    /// at:
    /// - https://github.com/graphprotocol/solidity-bindgen/blob/master/solidity-bindgen/src/secrets.rs
    /// - or https://crates.io/crates/zeroize
    /// if you care enough about your secrets to be used securely.
    ///
    /// If it's enough to pass a reference to `SecretKey` (lifetimes) than you can use `SecretKeyRef`
    /// wrapper.
    pub trait Key {
        /// Sign given message and include chain-id replay protection.
        ///
        /// When a chain ID is provided, the `Signature`'s V-value will have chain relay
        /// protection added (as per EIP-155). Otherwise, the V-value will be in
        /// 'Electrum' notation.
        fn sign(&self, message: &[u8], chain_id: Option<u64>) -> Result<Signature, SigningError>;

        /// Get public address that this key represents.
        fn address(&self) -> Address;
    }

    /// A `SecretKey` reference wrapper.
    ///
    /// A wrapper around `secp256k1::SecretKey` reference, which enables it to be used in methods expecting
    /// `Key` capabilities.
    pub struct SecretKeyRef<'a> {
        pub(super) key: &'a SecretKey,
    }

    impl<'a> SecretKeyRef<'a> {
        /// A simple wrapper around a reference to `SecretKey` which allows it to be usable for signing.
        pub fn new(key: &'a SecretKey) -> Self {
            Self { key }
        }
    }

    impl<'a> From<&'a SecretKey> for SecretKeyRef<'a> {
        fn from(key: &'a SecretKey) -> Self {
            Self::new(key)
        }
    }

    impl<'a> Deref for SecretKeyRef<'a> {
        type Target = SecretKey;

        fn deref(&self) -> &Self::Target {
            &self.key
        }
    }

    impl<T: Deref<Target = SecretKey>> Key for T {
        fn sign(&self, message: &[u8], chain_id: Option<u64>) -> Result<Signature, SigningError> {
            let message = Message::from_slice(&message).map_err(|_| SigningError::InvalidMessage)?;
            let (recovery_id, signature) = CONTEXT.sign_recoverable(&message, self).serialize_compact();

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

            Ok(Signature { v, r, s })
        }

        fn address(&self) -> Address {
            secret_key_address(self)
        }
    }

    /// Recover a sender, given message and the signature.
    ///
    /// Signature and `recovery_id` can be obtained from `types::Recovery` type.
    pub fn recover(message: &[u8], signature: &[u8], recovery_id: i32) -> Result<Address, RecoveryError> {
        let message = Message::from_slice(message).map_err(|_| RecoveryError::InvalidMessage)?;
        let recovery_id = RecoveryId::from_i32(recovery_id).map_err(|_| RecoveryError::InvalidSignature)?;
        let signature =
            RecoverableSignature::from_compact(&signature, recovery_id).map_err(|_| RecoveryError::InvalidSignature)?;
        let public_key = CONTEXT
            .recover(&message, &signature)
            .map_err(|_| RecoveryError::InvalidSignature)?;

        Ok(public_key_address(&public_key))
    }

    /// Gets the address of a public key.
    ///
    /// The public address is defined as the low 20 bytes of the keccak hash of
    /// the public key. Note that the public key returned from the `secp256k1`
    /// crate is 65 bytes long, that is because it is prefixed by `0x04` to
    /// indicate an uncompressed public key; this first byte is ignored when
    /// computing the hash.
    pub(crate) fn public_key_address(public_key: &PublicKey) -> Address {
        let public_key = public_key.serialize_uncompressed();

        debug_assert_eq!(public_key[0], 0x04);
        let hash = keccak256(&public_key[1..]);

        Address::from_slice(&hash[12..])
    }

    /// Gets the public address of a private key.
    pub(crate) fn secret_key_address(key: &SecretKey) -> Address {
        let secp = &*CONTEXT;
        let public_key = PublicKey::from_secret_key(secp, key);
        public_key_address(&public_key)
    }
}

/// A struct that represents the components of a secp256k1 signature.
pub struct Signature {
    /// V component in electrum format with chain-id replay protection.
    pub v: u64,
    /// R component of the signature.
    pub r: H256,
    /// S component of the signature.
    pub s: H256,
}

/// Compute the Keccak-256 hash of input bytes.
pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    use tiny_keccak::{Hasher, Keccak};
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    hasher.finalize(&mut output);
    output
}
