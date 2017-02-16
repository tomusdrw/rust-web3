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
  let web3 = web3::Web3::new(web3::transports::Http::new("http://localhost:8545").unwrap());
  let accounts = web3.eth().accounts().wait().unwrap();

  println!("Accounts: {:?}", accounts);
}
```

For more see [examples folder](./examples).

# TODO

## General
- [ ] More flexible API (accept `Into<X>`)
- [x] Contract calls (ABI encoding; `debris/ethabi`)
- [ ] Batch Requests

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
- [ ] Work type (`Work` from Parity)
- [ ] Syncing type (`SyncStats` from Parity)

## APIs
- [x] Eth: `eth_*`
- [ ] Eth filters: `eth_*`
- [x] `net_*`
- [x] `web3_*`
- [ ] `personal_*`
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
