//! Ethereum Name Service Interface
//!
//! This interface provides most fonctions implemented in ENS.
//! With it you can resolve ethereum addresses to domain names, domain name to blockchain adresses and more!
//!
//! # Example
//! ```no_run
//! # use eth_ens::Ens;
//! # use crate::transport::{Eip1193, Provider};
//! #
//! #[tokio::main]
//! async fn main() {
//!     let provider = Provider::default().unwrap().unwrap();
//!     let transport = Eip1193::new(provider);
//!     let ens = Ens::new(transport);
//!
//!     let addess = ens.eth_address("vitalik.eth").await.unwrap();
//! }
//! ```

mod eth_ens;
pub mod public_resolver;
pub mod registry;
pub mod reverse_resolver;

pub use eth_ens::Ens;
