//! Program error types.

use {
    num_derive::FromPrimitive,
    num_enum::TryFromPrimitive,
    solana_program_error::{ProgramError, ToStr},
    solana_pubkey::PubkeyError,
    thiserror::Error,
};

/// Errors that can be returned by the Config program.
#[derive(Error, Clone, Debug, Eq, PartialEq, FromPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum AddressLookupTableError {
    // Reimplementations of `PubkeyError` variants.
    //
    // Required for the BPF version since the Agave SDK only maps `PubkeyError`
    // to `ProgramError`, not `InstructionError`. Therefore, the builtin
    // version throws unknown custom error codes (0x0 - 0x2).
    /// Length of the seed is too long for address generation
    #[error("Length of the seed is too long for address generation")]
    PubkeyErrorMaxSeedLengthExceeded = 0,
    #[error("Provided seeds do not result in a valid address")]
    PubkeyErrorInvalidSeeds,
    #[error("Provided owner is not allowed")]
    PubkeyErrorIllegalOwner,
    /// Instruction modified data of a read-only account.
    #[error("Instruction modified data of a read-only account")]
    ReadonlyDataModified = 10, // Avoid collisions with System.
    /// Instruction changed the balance of a read-only account.
    #[error("Instruction changed the balance of a read-only account")]
    ReadonlyLamportsChanged,
}

impl ToStr for AddressLookupTableError {
    fn to_str(&self) -> &'static str {
        match self {
            Self::PubkeyErrorMaxSeedLengthExceeded => {
                "Length of the seed is too long for address generation"
            }
            Self::PubkeyErrorInvalidSeeds => "Provided seeds do not result in a valid address",
            Self::PubkeyErrorIllegalOwner => "Provided owner is not allowed",
            Self::ReadonlyDataModified => "Instruction modified data of a read-only account",
            Self::ReadonlyLamportsChanged => {
                "Instruction changed the balance of a read-only account"
            }
        }
    }
}

impl From<AddressLookupTableError> for ProgramError {
    fn from(e: AddressLookupTableError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl From<PubkeyError> for AddressLookupTableError {
    fn from(e: PubkeyError) -> Self {
        match e {
            PubkeyError::MaxSeedLengthExceeded => {
                AddressLookupTableError::PubkeyErrorMaxSeedLengthExceeded
            }
            PubkeyError::InvalidSeeds => AddressLookupTableError::PubkeyErrorInvalidSeeds,
            PubkeyError::IllegalOwner => AddressLookupTableError::PubkeyErrorIllegalOwner,
        }
    }
}
