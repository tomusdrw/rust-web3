use std::time::Duration;

use ethabi::Address;
use web3::{api::Eth, transports::WebSocket};

#[tokio::main]
async fn main() -> web3::Result<()> {
    let _ = env_logger::try_init();
    let transport = web3::transports::WebSocket::new("ws://localhost:8545").await?;
    let web3 = web3::Web3::new(transport);

    println!("Calling accounts.");
    let accounts = web3.eth().accounts().await?;

    interval_balance(&web3.eth(), accounts[0]).await;

    Ok(())
}

async fn interval_balance(eth: &Eth<WebSocket>, account: Address) {
    loop {
        match eth.balance(account, None).await {
            Ok(balance) => println!("Balance of {:?}: {}", account, balance),
            Err(e) => println!("Get balance failed: {}", e),
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
