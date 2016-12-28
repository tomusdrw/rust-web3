# rust-web3

Ethereum JSON-RPC multi-transport client.
Rust implementation of Web3.js library.

[![Build Status][travis-image]][travis-url]

[travis-image]: https://travis-ci.org/tomusdrw/rust-web3.svg?branch=master
[travis-url]: https://travis-ci.org/tomusdrw/rust-web3

[Documentation](http://tomusdrw.github.io/rust-web3/index.html)

# Examples
```rust
extern crate futures;
extern crate web3;

use futures::Future;
use web3::api::Eth;

fn main() {
  let web3 = web3::Web3::new(web3::transports::Http::new("http://localhost:8545").unwrap());
  let accounts = web3.eth().accounts().wait().unwrap();

  println!("Accounts: {:?}", accounts);
}
```

For more see [examples folder](./examples).

# TODO

## General
- [ ] Contract calls (ABI encoding; `debris/ethabi`)
- [ ] Batch Requests

## Transports
- [x] HTTP transport
- [ ] IPC transport
- [ ] WebSockets transport

## Types
- [ ] Types for `U256,H256,Address(H160)` from `ethcore/bigint` crate
- [ ] Index type (numeric, encoded to hex)
- [ ] Transaction type (`Transaction` from Parity)
- [ ] Transaction receipt type (`TransactionReceipt` from Parity)
- [ ] Block type (`RichBlock` from Parity)
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
