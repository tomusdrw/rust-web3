
use web3::futures::{future, StreamExt};

fn main() -> web3::Result<()> {
    web3::block_on(run())
}
async fn run() -> web3::Result<()> {
    let ws = web3::transports::WebSocket::new("ws://localhost:8546")?;
    let web3 = web3::Web3::new(ws.clone());
    let mut sub = web3.eth_subscribe().subscribe_new_heads().await?;

    println!("Got subscription id: {:?}", sub.id());

    (&mut sub)
        .take(5)
        .for_each(|x| {
            println!("Got: {:?}", x);
            future::ready(())
        })
        .await;

    sub.unsubscribe();

    Ok(())
}
