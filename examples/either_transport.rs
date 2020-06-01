
pub type Transport = web3::EitherTransport<web3::transports::Ipc, web3::transports::Http>;

fn main() -> Result<(), web3::Error> {
    let transport = web3::transports::Ipc::new("./jsonrpc.ipc")?;

    web3::block_on(run(web3::EitherTransport::Left(transport)))
}

async fn run(transport: Transport) -> Result<(), web3::Error> {
    let web3 = web3::Web3::new(transport);

    println!("Calling accounts.");
    let accounts = web3.eth().accounts().await?;
    println!("Accounts: {:?}", accounts);

    println!("Calling balance.");
    let balance = web3.eth().balance("0x0".parse().unwrap(), None).await?;
    println!("Balance: {}", balance);

    Ok(())
}
