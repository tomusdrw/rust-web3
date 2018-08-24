use types::{Bytes, H160, H256, U256};
use rlp::RlpStream;
use tiny_keccak::keccak256;
use secp256k1::key::SecretKey;
use secp256k1::Message;
use secp256k1::Secp256k1;

/// Description of a Transaction, pending or in the chain.
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct RawTransaction {
    /// Nonce
    pub nonce: U256,
    /// Recipient (None when contract creation)
    pub to: Option<H160>,
    /// Transfered value
    pub value: U256,
    /// Gas Price
    #[serde(rename = "gasPrice")]
    pub gas_price: U256,
    /// Gas amount
    pub gas: U256,
    /// Input data
    pub data: Bytes
}

impl RawTransaction {
    /// Signs and returns the RLP-encoded transaction
    pub fn sign(&self, private_key: &H256) -> Bytes {
        let hash = self.hash();
        let sig = ecdsa_sign(&hash, &private_key.0);
        let mut tx = RlpStream::new(); 
        tx.begin_unbounded_list();
        self.encode(&mut tx);
        tx.append(&sig.v); 
        tx.append(&sig.r); 
        tx.append(&sig.s); 
        tx.complete_unbounded_list();
        Bytes(tx.out())
    }

    fn hash(&self) -> Vec<u8> {
        let mut hash = RlpStream::new(); 
        hash.begin_unbounded_list();
        self.encode(&mut hash);
        hash.complete_unbounded_list();
        keccak256_hash(&hash.out())
    }

    fn encode(&self, s: &mut RlpStream) {
        s.append(&self.nonce);
        s.append(&self.gas_price);
        s.append(&self.gas);
        s.append(&self.to.unwrap());
        s.append(&self.value);
        s.append(&self.data.0);
    }
}

fn keccak256_hash(bytes: &[u8]) -> Vec<u8> {
    keccak256(bytes).into_iter().cloned().collect()
}

fn ecdsa_sign(hash: &[u8], private_key: &[u8]) -> EcdsaSig {
    let s = Secp256k1::signing_only();
    let msg = Message::from_slice(hash).unwrap();
    let key = SecretKey::from_slice(&s, private_key).unwrap();
    let sig_bytes = s.sign(&msg, &key).serialize_compact(&s).to_vec();
    EcdsaSig {
        v: vec![0x1c],
        r: sig_bytes[0..32].to_vec(),
        s: sig_bytes[32..64].to_vec(),
    }
}

pub struct EcdsaSig {
    v: Vec<u8>,
    r: Vec<u8>,
    s: Vec<u8>
}

mod test {
    const TEST_TX: &'static str = "\"0xf8640182200083100000940000000000000000000000000000000000000000821000801ca0f20da6484a64d2409581686b70beba08b4a10fec189dcaa0c4239ed9b7b1aa9ca06ab517f8c43f02e94b84275f6dcdd2d162ce038040e4f643d61eebdc45ea9bde\"";

    #[test]
    fn test_signs_transaction() {
        use types::*;
        use std::str::FromStr;
        use serde_json;
        let tx = RawTransaction {
            nonce: (1).into(),
            gas_price: U256::from_str("2000").unwrap(),
            gas: U256::from_str("100000").unwrap(),
            to: Some((0u64).into()),
            value: U256::from_str("1000").unwrap(),
            data: Bytes(vec![])
        };
        let raw_tx = tx.sign(&H256::from_str("c0dec0dec0dec0dec0dec0dec0dec0dec0dec0dec0dec0dec0dec0dec0dec0de").unwrap());
        let bytes: Bytes = serde_json::from_str(TEST_TX).unwrap();
        assert_eq!(bytes, raw_tx);
    }
}

