extern crate web3;

use web3::futures::{Future, Stream};

fn main() {
    let (_eloop, ws) = web3::transports::WebSocket::new("ws://localhost:8546").unwrap();
    let web3 = web3::Web3::new(ws.clone());
    let mut sub = web3.eth_subscribe().subscribe_new_heads().wait().unwrap();

    println!("Got subscription id: {:?}", sub.id());

    (&mut sub)
        .take(5)
        .for_each(|x| {
            println!("Got: {:?}", x);
            Ok(())
        })
        .wait()
        .unwrap();

    sub.unsubscribe();

    drop(web3);
}
