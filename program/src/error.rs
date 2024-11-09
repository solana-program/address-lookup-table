//! Program error types.

use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

/// Errors that can be returned by the Config program.
#[derive(Error, Clone, Debug, Eq, PartialEq, FromPrimitive)]
pub enum AddressLookupTableError {
    /// Instruction modified data of a read-only account.
    #[error("Instruction modified data of a read-only account")]
    ReadonlyDataModified = 10, // Avoid collisions with System.
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
