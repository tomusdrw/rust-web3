//! Ethereum Name Service Interface
//!
//! This interface provides most functions implemented in ENS.
//! With it you can resolve ethereum addresses to domain names, domain name to blockchain addresses and more!
//!
//! # Example
//! ```no_run
//! ##[tokio::main]
//! async fn main() -> web3::Result<()> {
//!     use crate::web3::api::Namespace;
//!
//!     let transport = web3::transports::Http::new("http://localhost:8545")?;
//!     
//!     let ens = web3::contract::ens::Ens::new(transport);
//!
//!     let address = ens.eth_address("vitalik.eth").await.unwrap();
//!
//!     println!("Address: {:?}", address);
//!
//!     Ok(())
//! }
//! ```

mod eth_ens;
pub mod public_resolver;
pub mod registry;
pub mod reverse_resolver;

pub use eth_ens::Ens;
