extern crate web3;

fn main() -> web3::Result<()> {
    web3::block_on(run())
}

async fn run() -> web3::Result<()> {
    let ipc = web3::transports::Ipc::new("./jsonrpc.ipc")?;
    let web3 = web3::Web3::new(ipc);
    println!("Calling accounts.");

    let accounts = web3.eth().accounts().await?;
    println!("Accounts: {:?}", accounts);

    Ok(())
}
