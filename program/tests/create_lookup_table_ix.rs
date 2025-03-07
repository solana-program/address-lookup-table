#![cfg(feature = "test-sbf")]

mod common;

use {
    common::setup,
    mollusk_svm::{program::keyed_account_for_system_program, result::Check},
    solana_address_lookup_table_program::{
        instruction::create_lookup_table,
        state::{AddressLookupTable, LOOKUP_TABLE_META_SIZE},
    },
    solana_sdk::{
        account::{Account, ReadableAccount},
        clock::Slot,
        program_error::ProgramError,
        pubkey::Pubkey,
        rent::Rent,
        system_program,
    },
};

// [Core BPF]: Tests that assert proper authority checks have been removed,
// since feature "FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap"
// (relax_authority_signer_check_for_lookup_table_creation) has been activated
// on all clusters.

#[test]
fn test_create_lookup_table_idempotent() {
    let mut mollusk = setup();

    let test_recent_slot = 123;

    // [Core BPF]: Warping to slot, which will update `SlotHashes`.
    mollusk.warp_to_slot(test_recent_slot + 1);

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let (create_lookup_table_ix, lookup_table_address) =
        create_lookup_table(authority, payer, test_recent_slot);

    // First create should succeed
    let result = mollusk.process_and_validate_instruction(
        &create_lookup_table_ix,
        &[
            (lookup_table_address, Account::default()),
            (authority, Account::default()),
            (payer, Account::new(100_000_000, 0, &system_program::id())),
            keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );

    let lookup_table_account = result.get_account(&lookup_table_address).unwrap();

    assert_eq!(
        lookup_table_account.owner(),
        &solana_address_lookup_table_program::id()
    );
    assert_eq!(lookup_table_account.data().len(), LOOKUP_TABLE_META_SIZE);
    assert_eq!(
        lookup_table_account.lamports(),
        Rent::default().minimum_balance(LOOKUP_TABLE_META_SIZE)
    );
    let lookup_table = AddressLookupTable::deserialize(lookup_table_account.data()).unwrap();
    assert_eq!(lookup_table.meta.deactivation_slot, Slot::MAX);
    assert_eq!(lookup_table.meta.authority, Some(authority));
    assert_eq!(lookup_table.meta.last_extended_slot, 0);
    assert_eq!(lookup_table.meta.last_extended_slot_start_index, 0);
    assert_eq!(lookup_table.addresses.len(), 0);

    // Second create should succeed too
    mollusk.process_and_validate_instruction(
        &create_lookup_table_ix,
        &[
            (lookup_table_address, lookup_table_account.clone()),
            (authority, Account::default()),
            (payer, Account::default()), // Note the lack of lamports.
            keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
}

#[test]
fn test_create_lookup_table_use_payer_as_authority() {
    let mut mollusk = setup();

    let test_recent_slot = 123;

    // [Core BPF]: Warping to slot, which will update `SlotHashes`.
    mollusk.warp_to_slot(test_recent_slot + 1);

    let payer = Pubkey::new_unique();
    let payer_account = Account::new(100_000_000, 0, &system_program::id());

    let (create_lookup_table_ix, lookup_table_address) =
        create_lookup_table(payer, payer, test_recent_slot);

    mollusk.process_and_validate_instruction(
        &create_lookup_table_ix,
        &[
            (lookup_table_address, Account::default()),
            (payer, payer_account.clone()),
            (payer, payer_account),
            keyed_account_for_system_program(),
        ],
        &[Check::success()],
    );
}

#[test]
fn test_create_lookup_table_not_recent_slot() {
    let mollusk = setup();

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let (create_lookup_table_ix, lookup_table_address) =
        create_lookup_table(authority, payer, Slot::MAX);

    mollusk.process_and_validate_instruction(
        &create_lookup_table_ix,
        &[
            (lookup_table_address, Account::default()),
            (authority, Account::default()),
            (payer, Account::new(100_000_000, 0, &system_program::id())),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::InvalidInstructionData)],
    );
}

#[test]
fn test_create_lookup_table_pda_mismatch() {
    let mut mollusk = setup();

    let test_recent_slot = 123;

    // [Core BPF]: Warping to slot, which will update `SlotHashes`.
    mollusk.warp_to_slot(test_recent_slot + 1);

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let wrong_pda = Pubkey::new_unique();
    let mut create_lookup_table_ix = create_lookup_table(authority, payer, test_recent_slot).0;
    create_lookup_table_ix.accounts[0].pubkey = wrong_pda;

    mollusk.process_and_validate_instruction(
        &create_lookup_table_ix,
        &[
            (wrong_pda, Account::default()),
            (authority, Account::default()),
            (payer, Account::new(100_000_000, 0, &system_program::id())),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::InvalidArgument)],
    );
}
