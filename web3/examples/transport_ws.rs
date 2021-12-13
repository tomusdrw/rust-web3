use hex_literal::hex;

#[tokio::main]
async fn main() -> web3::Result<()> {
    let _ = env_logger::try_init();
    let transport = web3::transports::WebSocket::new("ws://localhost:8546").await?;
    let web3 = web3::Web3::new(transport);

    println!("Calling accounts.");
    let mut accounts = web3.eth().accounts().await?;
    println!("Accounts: {:?}", accounts);
    accounts.push(hex!("00a329c0648769a73afac7f9381e08fb43dbea72").into());

    println!("Calling balance.");
    for account in accounts {
        let balance = web3.eth().balance(account, None).await?;
        println!("Balance of {:?}: {}", account, balance);
    }

    Ok(())
}
