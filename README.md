# web3

Ethereum JSON-RPC multi-transport client.
Rust implementation of Web3.js library.

[![Build Status][ci-image]][ci-url] [![Crates.io](https://img.shields.io/crates/v/web3)](https://crates.io/crates/web3)

[ci-image]: https://github.com/tomusdrw/rust-web3/workflows/Compilation%20and%20Testing%20Suite/badge.svg
[ci-url]: https://github.com/tomusdrw/rust-web3/actions?query=workflow%3A%22Compilation+and+Testing+Suite%22
[docs-rs-badge]: https://docs.rs/web3/badge.svg
[docs-rs-url]: https://docs.rs/web3

Documentation: [crates.io][docs-rs-url]

## Status

Note this package is **barely maintained** and I am looking for an active maintainer (see #664).
If you are starting a new project, I'd recommend choosing https://github.com/gakonst/ethers-rs instead.

## Usage

First, add this to your `Cargo.toml`:

```toml
[dependencies]
web3 = "0.19.0"
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

## Futures migration
- [ ] Get rid of parking_lot (replace with async-aware locks if really needed).
- [ ] Consider getting rid of `Unpin` requirements. (#361)
- [x] WebSockets: TLS support (#360)
- [ ] WebSockets: Reconnecting & Pings
- [x] Consider using `tokio` instead of `async-std` for `ws.rs` transport (issue with test).
- [x] Restore IPC Transport

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
To compile, you need to disable the IPC feature:
```toml
web3 = { version = "_", default-features = false, features = ["http"] }
```

# Avoiding OpenSSL dependency

On Linux, `native-tls` is implemented using OpenSSL. To avoid that dependency
for HTTPS or WSS use the corresponding features.
```toml
web3 = { version = "_", default-features = false, features = ["http-rustls-tls", "ws-rustls-tokio"] }
```

_Note: To fully replicate the default features also add `signing` & `ipc-tokio` features_.

# Cargo Features

The library supports following features:
- `http` - Enables HTTP transport (requires `tokio` runtime, because of `hyper`).
- `http-tls` - Enables TLS support via `reqwest/default-tls` for HTTP transport (implies `http`; default).
- `http-native-tls` - Enables TLS support via `reqwest/native-tls` for HTTP transport (implies `http`).
- `http-rustls-tls` - Enables TLS support via `reqwest/rustls-tls` for HTTP transport (implies `http`).
- `ws-tokio` - Enables WS transport using `tokio` runtime.
- `ws-tls-tokio` - Enables TLS support for WS transport (implies `ws-tokio`; default).
- `ws-rustls-tokio` - Enables rustls TLS support for WS transport (implies `ws-tokio`).
- `ws-async-std` - Enables WS transport using `async-std` runtime.
- `ws-tls-async-std` - Enables TLS support for WS transport (implies `ws-async-std`).
- `ipc-tokio` - Enables IPC transport using `tokio` runtime (default).
- `signing` - Enable account namespace and local-signing support (default).
- `eip-1193` - Enable EIP-1193 support.
- `wasm` - Compile for WASM (make sure to disable default features).
- `arbitrary_precision` - Enable `arbitrary_precision` in `serde_json`.
- `allow-missing-fields` - Some response fields are mandatory in Ethereum but not present in
  EVM-compatible chains such as Celo and Fantom. This feature enables compatibility by setting a
  default value on those fields.
