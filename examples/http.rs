extern crate web3;

fn main() {
    let web3 = web3::Web3::new(
        web3::transports::Http::new("http://localhost:8545").unwrap()
    );
    let accounts = web3.eth().accounts();
    let accounts = web3::block_on(accounts).unwrap();
    println!("Accounts: {:?}", accounts);
}
