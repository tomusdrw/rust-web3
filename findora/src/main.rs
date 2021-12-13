use secp256k1::SecretKey;
use std::str::FromStr;
use web3::types::{Address, TransactionId, TransactionParameters, U256};

//const FRC20_ADDRESS: u64 = 0x1000;

#[tokio::main]
async fn main() -> web3::Result<()> {
    let transport = web3::transports::Http::new("https://dev-qa01.dev.findora.org:8545")?;
    let web3 = web3::Web3::new(transport);

    //println!("chain_id {}", web3.eth().chain_id().await?);
    //println!("gas_price {}", web3.eth().gas_price().await?);
    //println!("block_number {}", web3.eth().block_number().await?);
    //println!(
    //    "frc20 code {:?}",
    //    web3.eth().code(H160::from_low_u64_be(FRC20_ADDRESS), None).await?
    //);

    let to = Address::from_str("0x6cD65d32f778b639Ea5656Ba77994319d89bB5AE").unwrap();
    let sk = SecretKey::from_str("b8836c243a1ff93a63b12384176f102345123050c9f3d3febbb82e3acd6dd1cb").unwrap();
    // Build the tx object
    let tx_object = TransactionParameters {
        to: Some(to),
        value: U256::exp10(17), //0.1 eth
        ..Default::default()
    };

    // Sign the tx (can be done offline)
    let signed = web3.accounts().sign_transaction(tx_object, &sk).await?;

    // Send the tx to infura
    let result = web3.eth().send_raw_transaction(signed.raw_transaction).await?;

    println!("Tx succeeded with hash: {}", result);
    println!("Tx receipt: {:?}", web3.eth().transaction_receipt(result).await?);
    println!("Tx {:?}", web3.eth().transaction(TransactionId::Hash(result)).await?);

    println!("Calling accounts.");
    let mut accounts = web3.eth().accounts().await?;
    println!("Accounts: {:?}", accounts);
    accounts.push("Bb4a0755b740a55Bf18Ac4404628A1a6ae8B6F8F".parse().unwrap());
    accounts.push("6cD65d32f778b639Ea5656Ba77994319d89bB5AE".parse().unwrap());

    println!("Calling balance.");
    for account in accounts {
        let balance = web3.eth().balance(account, None).await?;
        println!("Balance of {:?}: {}", account, balance);
    }

    Ok(())
}
