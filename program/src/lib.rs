//! Address Lookup Table Program
// [Core BPF]: Required for `solana-frozen-abi-macro` to work.
#![allow(incomplete_features)]
#![cfg_attr(RUSTC_WITH_SPECIALIZATION, feature(specialization))]

#[cfg(all(target_os = "solana", feature = "bpf-entrypoint"))]
mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

solana_program::declare_id!("AddressLookupTab1e1111111111111111111111111");
