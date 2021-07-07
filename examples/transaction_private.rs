use std::str::FromStr;

use web3::{
    ethabi::ethereum_types::U256,
    types::{Address, TransactionRequest},
};

/// Below sends a transaction to a local node that stores private keys (eg Ganache)
/// For generating and signing a transaction offline, before transmitting it to a public node (eg Infura) see transaction_public
#[tokio::main]
async fn main() -> web3::Result {
    let transport = web3::transports::Http::new("http://localhost:7545")?;
    let web3 = web3::Web3::new(transport);

    // Insert the 20-byte "from" address in hex format (prefix with 0x)
    let from = Address::from_str("0xC48ad5fd060e1400a41bcf51db755251AD5A2475").unwrap();

    // Insert the 20-byte "to" address in hex format (prefix with 0x)
    let to = Address::from_str("0xCd550E94040cEC1b33589eB99B0E1241Baa75D19").unwrap();

    // Build the tx object
    let tx_object = TransactionRequest {
        from,
        to: Some(to),
        value: Some(U256::exp10(17)), //0.1 eth
        ..Default::default()
    };

    // Send the tx to localhost
    let result = web3.eth().send_transaction(tx_object).await?;

    println!("Tx succeeded with hash: {}", result);

    Ok(())
}
