
async fn run() -> Result<(), web3::Error> {
    let http = web3::transports::Http::new("http://localhost:8545")?;
    let web3 = web3::Web3::new(http);
    let accounts = web3.eth().accounts().await?;

    println!("Accounts: {:?}", accounts);
    Ok(())
}

fn main() -> Result<(), web3::Error> {
    web3::block_on(run())
}
