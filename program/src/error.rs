//! Program error types.

use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
        pubkey::PubkeyError,
    },
    thiserror::Error,
};

/// Errors that can be returned by the Config program.
#[derive(Error, Clone, Debug, Eq, PartialEq, FromPrimitive)]
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

impl PrintProgramError for AddressLookupTableError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<AddressLookupTableError> for ProgramError {
    fn from(e: AddressLookupTableError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for AddressLookupTableError {
    fn type_of() -> &'static str {
        "AddressLookupTableError"
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
