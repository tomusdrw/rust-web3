use web3::futures::{future, StreamExt};

#[tokio::main]
async fn main() -> web3::Result {
    let ws = web3::transports::WebSocket::new("ws://localhost:8546").await?;
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
