extern crate web3;

fn main() -> web3::Result<()> {
    web3::block_on(run())
}

async fn run() -> web3::Result<()> {
    let transport = web3::transports::Ipc::new("./jsonrpc.ipc")?;
    let web3 = web3::Web3::new(transport);

    println!("Calling accounts.");
    let accounts = web3.eth().accounts().await?;
    println!("Accounts: {:?}", accounts);

    println!("Calling balance.");
    let balance = web3.eth().balance("0x0".parse().unwrap(), None).await?;
    println!("Balance: {}", balance);

    Ok(())
}
