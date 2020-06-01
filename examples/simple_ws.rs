
fn main() -> web3::Result<()> {
    let ws = web3::transports::WebSocket::new("ws://localhost:8546")?;
    let web3 = web3::Web3::new(ws);
    let accounts = futures::executor::block_on(web3.eth().accounts())?;

    println!("Accounts: {:?}", accounts);
    Ok(())
}
