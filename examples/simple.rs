#[tokio::main]
async fn main() -> web3::Result {
    let http = web3::transports::Http::new("http://localhost:8545")?;
    let web3 = web3::Web3::new(http);
    let accounts = web3.eth().accounts().await?;

    println!("Accounts: {:?}", accounts);
    Ok(())
}
