//! Program entrypoint

use {
    crate::{error::AddressLookupTableError, processor},
    solana_account_info::AccountInfo,
    solana_msg::msg,
    solana_program_error::{ProgramResult, ToStr},
    solana_pubkey::Pubkey,
};

solana_program_entrypoint::entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = processor::process(program_id, accounts, instruction_data) {
        msg!(error.to_str::<AddressLookupTableError>());
        return Err(error);
    }
    Ok(())
}
