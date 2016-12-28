# rust-web3
Rust Ethereum JSON-RPC client (Web3).

[Documentation](http://tomusdrw.github.io/rust-web3/index.html)

# Examples
See [Examples folder](./examples).

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
