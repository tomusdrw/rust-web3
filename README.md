# rust-web3

Ethereum JSON-RPC multi-transport client.
Rust implementation of Web3.js library.

[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/tomusdrw/rust-web3.svg?branch=master
[travis-url]: https://travis-ci.org/tomusdrw/rust-web3

[Documentation](http://tomusdrw.github.io/rust-web3/index.html)

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies]
web3 = { git = "https://github.com/tomusdrw/rust-web3" }
```

## Example
```rust
#[tokio::main]
async fn main() -> web3::Result<()> {
    let transport = web3::transports::Http::new("http://localhost:8545")?;
    let web3 = web3::Web3::new(transport);

    println!("Calling accounts.");
    let mut accounts = web3.eth().accounts().await?;
    println!("Accounts: {:?}", accounts);
    accounts.push("00a329c0648769a73afac7f9381e08fb43dbea72".parse().unwrap());

    println!("Calling balance.");
    for account in accounts {
        let balance = web3.eth().balance(account, None).await?;
        println!("Balance of {:?}: {}", account, balance);
    }

    Ok(())
}
```

If you want to deploy smart contracts you have written you can do something like this (make sure you have the solidity compiler installed):

`solc -o build --bin --abi contracts/*.sol`

The solidity compiler is generating the binary and abi code for the smart contracts in a directory called contracts and is being output to a directory called build.

For more see [examples folder](./examples).

# Opt-out Features
- `http` - Enables HTTP transport (requires `tokio` runtime, because of `hyper`).
- `http-tls` - Enables TLS support for HTTP transport (implies `http`).
- `ws` - Enables WS transport.
- `ws-tls` - Enables TLS support for WS transport (implies `ws`).

## Futures migration
- [ ] Get rid of parking_lot (replace with async-aware locks if really needed).
- [ ] Consider getting rid of `Unpin` requirements. (#361)
- [x] WebSockets: TLS support (#360)
- [ ] WebSockets: Reconnecting & Pings
- [ ] Consider using `tokio` instead of `async-std` for `ws.rs` transport (issue with test).
- [ ] Restore IPC Transport

## General
- [ ] More flexible API (accept `Into<X>`)
- [x] Contract calls (ABI encoding; `debris/ethabi`)
- [X] Batch Requests

## Transports
- [x] HTTP transport
- [x] IPC transport
- [x] WebSockets transport

## Types
- [x] Types for `U256,H256,Address(H160)`
- [x] Index type (numeric, encoded to hex)
- [x] Transaction type (`Transaction` from Parity)
- [x] Transaction receipt type (`TransactionReceipt` from Parity)
- [x] Block type (`RichBlock` from Parity)
- [x] Work type (`Work` from Parity)
- [X] Syncing type (`SyncStats` from Parity)

## APIs
- [x] Eth: `eth_*`
- [x] Eth filters: `eth_*`
- [x] Eth pubsub: `eth_*`
- [x] `net_*`
- [x] `web3_*`
- [x] `personal_*`
- [ ] `traces_*`

### Parity-specific APIs
- [ ] Parity read-only: `parity_*`
- [ ] Parity accounts: `parity_*` (partially implemented)
- [x] Parity set: `parity_*`
- [ ] `signer_*`

- [x] Own APIs (Extendable)
```rust
let web3 = Web3::new(transport);
web3.api::<CustomNamespace>().custom_method().wait().unwrap()
```

# Installation on Windows

Currently, Windows does not support IPC, which is enabled in the library by default.
To complile, you need to disable the IPC feature:
```
web3 = { version = "0.11.0", default-features = false, features = ["http"] }
```
