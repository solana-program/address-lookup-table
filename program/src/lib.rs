pub mod assertions;
pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;

pub use solana_program;

solana_program::declare_id!("AddressLookupTab1e1111111111111111111111111");
