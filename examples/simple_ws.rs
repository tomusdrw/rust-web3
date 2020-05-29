extern crate web3;

use web3::futures::Future;

fn main() {
    let (_eloop, ws) = web3::transports::WebSocket::new("ws://localhost:8546").unwrap();
    let web3 = web3::Web3::new(ws);
    let accounts = futures::executor::block_on(web3.eth().accounts()).unwrap();

    println!("Accounts: {:?}", accounts);
}
