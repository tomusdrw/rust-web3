#![cfg(target_arch = "wasm32")]

use wasm_bindgen::*;

use wasm_bindgen_test::{console_log, wasm_bindgen_test};
wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

use web3::{
    api::Namespace,
    contract::ens::Ens,
    transports::eip_1193::{Eip1193, Provider},
    types::{Address, U256},
};

use hex_literal::hex;

fn get_ens() -> Ens<Eip1193> {
    let provider = Provider::default().expect("Get Provider").expect("Provider is None");
    let transport = Eip1193::new(provider);
    let ens = Ens::new(transport);

    ens
}

#[wasm_bindgen_test]
async fn owner() {
    let ens = get_ens();

    let domain = "vitalik.eth";

    let addr = ens.owner(domain).await.unwrap();

    console_log!("Owner of {} => {:?}", domain, addr);

    assert_eq!(addr, Address::from(hex!("d8da6bf26964af9d7eed9e03e53415d37aa96045")))
}

#[wasm_bindgen_test]
async fn resolver() {
    let ens = get_ens();

    let domain = "vitalik.eth";

    let addr = ens.resolver(domain).await.unwrap();

    console_log!("Resolver of {} => {:?}", domain, addr);

    //ENS: Public resolver 2
    assert_eq!(addr, Address::from(hex!("4976fb03c32e5b8cfe2b6ccb31c09ba78ebaba41")))
}

#[wasm_bindgen_test]
async fn ttl() {
    let ens = get_ens();

    let domain = "vitalik.eth";

    let ttl = ens.ttl(domain).await.unwrap();

    console_log!("TTL of {} => {}", domain, ttl);
}

#[wasm_bindgen_test]
async fn record_exists() {
    let ens = get_ens();

    let domain = "vitalik.eth";

    let exist = ens.record_exists(domain).await.unwrap();

    console_log!("Record exist of {} => {}", domain, exist);

    assert!(exist)
}

#[wasm_bindgen_test]
async fn supports_interface() {
    let ens = get_ens();

    let domain = "brantly.eth";

    let support = ens.supports_interface(domain, [0xf1, 0xcb, 0x7e, 0x06]).await.unwrap();

    console_log!("{} Support Interface => {}", domain, support);

    assert!(support)
}

#[wasm_bindgen_test]
async fn eth_address() {
    let ens = get_ens();

    let domain = "vitalik.eth";

    let addr = ens.eth_address(domain).await.unwrap();

    console_log!("ETH address of {} => {:?}", domain, addr);

    assert_eq!(addr, Address::from(hex!("d8da6bf26964af9d7eed9e03e53415d37aa96045")))
}

#[wasm_bindgen_test]
async fn blockchain_address() {
    let ens = get_ens();

    let domain = "brantly.eth";

    let addr = ens.blockchain_address(domain, U256::from(0u32)).await.unwrap();

    console_log!("BTC address of {} => {:?}", domain, addr);

    assert_eq!(addr, hex!("9010587f8364b964fcaa70687216b53bd2cbd798"))
}

#[wasm_bindgen_test]
async fn pubkey() {
    let ens = get_ens();

    let domain = "brantly.eth";

    let (x, y) = ens.pubkey(domain).await.unwrap();

    console_log!("PubKey of {} => {:?} {:?}", domain, x, y);
}

#[wasm_bindgen_test]
async fn content_hash() {
    let ens = get_ens();

    let domain = "vitalik.eth";

    let hash = ens.content_hash(domain).await.unwrap();

    console_log!("Content hash of {} => {:?}", domain, hash);
}

#[wasm_bindgen_test]
async fn text() {
    let ens = get_ens();

    let domain = "brantly.eth";
    let key = "location";

    let res = ens.text(domain, key.to_owned()).await.unwrap();

    console_log!("{} Text key {} => {}", domain, key, res);

    assert_eq!(res, "USA")
}

#[wasm_bindgen_test]
async fn canonical_name() {
    let ens = get_ens();

    let address = Address::from(hex!("983110309620D911731Ac0932219af06091b6744"));
    let domain = "brantly.eth";

    let res = ens.canonical_name(address).await.unwrap();

    console_log!("{:?} Name => {}", address, res);

    assert_eq!(res, domain)
}
