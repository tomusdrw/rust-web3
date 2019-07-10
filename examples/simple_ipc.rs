extern crate web3;

use web3::futures::Future;

fn main() {
    let (_el, transport) = web3::transports::Ipc::new("./jsonrpc.ipc").unwrap();
    let web3 = web3::Web3::new(transport);

    println!("Calling accounts.");
    let accounts = web3.eth().accounts().wait().unwrap();
    println!("Accounts: {:?}", accounts);

    println!("Calling balance.");
    let balance = web3.eth().balance("0x0".parse().unwrap(), None).wait().unwrap();
    println!("Balance: {}", balance);
}
