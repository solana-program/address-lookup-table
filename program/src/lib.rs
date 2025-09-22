//! Address Lookup Table Program
#![cfg_attr(feature = "frozen-abi", feature(min_specialization))]

#[cfg(target_os = "solana")]
mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

solana_pubkey::declare_id!("AddressLookupTab1e1111111111111111111111111");
