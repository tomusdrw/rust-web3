# rust-web3

Ethereum JSON-RPC multi-transport client.
Rust implementation of Web3.js library.

[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/tomusdrw/rust-web3.svg?branch=master
[travis-url]: https://travis-ci.org/tomusdrw/rust-web3

[Documentation](http://tomusdrw.github.io/rust-web3/index.html)

# Examples
```rust
extern crate web3;

use web3::futures::Future;

fn main() {
  let (_eloop, transport) = web3::transports::Http::new("http://localhost:8545").unwrap();
  let web3 = web3::Web3::new(transport);
  let accounts = web3.eth().accounts().wait().unwrap();

  println!("Accounts: {:?}", accounts);
}
```

If you want to deploy smart contracts you have written you can do something like this (make sure you have the solidity compiler installed):

`solc -o build --bin --abi contracts/*.sol`

The solidity compiler is generating the binary and abi code for the smart contracts in a directory called contracts and is being output to a directory called build.

For more see [examples folder](./examples).

## General
- [ ] More flexible API (accept `Into<X>`)
- [x] Contract calls (ABI encoding; `debris/ethabi`)
- [X] Batch Requests

## Transports
- [x] HTTP transport
- [x] IPC transport
- [ ] WebSockets transport

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
- [x] `net_*`
- [x] `web3_*`
- [x] `personal_*`
- [ ] `traces_*`

### Parity-specific APIs
- [ ] Parity read-only: `parity_*`
- [ ] Parity accounts: `parity_*`
- [ ] Parity set: `parity_*`
- [ ] `signer_*`

- [x] Own APIs (Extendable)
```rust
let web3 = Web3::new(transport);
web3.api::<CustomNamespace>().custom_method().wait().unwrap()
```

# Installation on Windows

Currently, Windows does not support IPC, which is enabled in the library by default.
To complile, you need to disable IPC feature:
```
web3 = { version = "0.1.0", default-features = false, features = ["http"] }
```
