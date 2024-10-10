#![cfg(feature = "test-sbf")]

mod common;

use {
    common::{lookup_table_account, new_address_lookup_table, setup},
    mollusk_svm::result::Check,
    solana_address_lookup_table_program::{
        instruction::freeze_lookup_table, state::AddressLookupTable,
    },
    solana_sdk::{
        account::{AccountSharedData, ReadableAccount},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

#[test]
fn test_freeze_lookup_table() {
    let mollusk = setup();

    let authority = Pubkey::new_unique();
    let mut initialized_table = new_address_lookup_table(Some(authority), 10);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table.clone());

    let result = mollusk.process_and_validate_instruction(
        &freeze_lookup_table(lookup_table_address, authority),
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
        ],
        &[Check::success()],
    );

    let lookup_table_account = result.get_account(&lookup_table_address).unwrap();
    let lookup_table = AddressLookupTable::deserialize(&lookup_table_account.data()).unwrap();

    assert_eq!(lookup_table.meta.authority, None);

    // Check that only the authority changed
    initialized_table.meta.authority = None;
    assert_eq!(initialized_table, lookup_table);
}

#[test]
fn test_freeze_immutable_lookup_table() {
    let mollusk = setup();

    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(None, 10);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    mollusk.process_and_validate_instruction(
        &freeze_lookup_table(lookup_table_address, authority),
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
        ],
        &[Check::err(ProgramError::Immutable)],
    );
}

#[test]
fn test_freeze_deactivated_lookup_table() {
    let mollusk = setup();

    let authority = Pubkey::new_unique();
    let initialized_table = {
        let mut table = new_address_lookup_table(Some(authority), 10);
        table.meta.deactivation_slot = 0;
        table
    };

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    mollusk.process_and_validate_instruction(
        &freeze_lookup_table(lookup_table_address, authority),
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
        ],
        &[Check::err(ProgramError::InvalidArgument)],
    );
}

#[test]
fn test_freeze_lookup_table_with_wrong_authority() {
    let mollusk = setup();

    let authority = Pubkey::new_unique();
    let wrong_authority = Pubkey::new_unique();

    let initialized_table = new_address_lookup_table(Some(authority), 10);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    mollusk.process_and_validate_instruction(
        &freeze_lookup_table(lookup_table_address, wrong_authority),
        &[
            (lookup_table_address, lookup_table_account),
            (wrong_authority, AccountSharedData::default()),
        ],
        &[Check::err(ProgramError::IncorrectAuthority)],
    );
}

#[test]
fn test_freeze_lookup_table_without_signing() {
    let mollusk = setup();

    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(Some(authority), 10);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    let mut instruction = freeze_lookup_table(lookup_table_address, authority);
    instruction.accounts[1].is_signer = false;

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
        ],
        &[Check::err(ProgramError::MissingRequiredSignature)],
    );
}

#[test]
fn test_freeze_empty_lookup_table() {
    let mollusk = setup();

    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(Some(authority), 0);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    mollusk.process_and_validate_instruction(
        &freeze_lookup_table(lookup_table_address, authority),
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
        ],
        &[Check::err(ProgramError::InvalidInstructionData)],
    );
}
