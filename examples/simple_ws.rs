#[tokio::main]
async fn main() -> web3::Result<()> {
    let ws = web3::transports::WebSocket::new("ws://localhost:8546")?;
    let web3 = web3::Web3::new(ws);
    let accounts = web3.eth().accounts().await?;
    println!("Accounts: {:?}", accounts);
    Ok(())
}
