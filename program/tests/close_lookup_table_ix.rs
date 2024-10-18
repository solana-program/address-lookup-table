#![cfg(feature = "test-sbf")]

mod common;

use {
    common::{lookup_table_account, new_address_lookup_table, setup},
    mollusk_svm::result::Check,
    solana_address_lookup_table_program::instruction::close_lookup_table,
    solana_sdk::{
        account::AccountSharedData, program_error::ProgramError, pubkey::Pubkey,
        slot_hashes::MAX_ENTRIES,
    },
};

#[test]
fn test_close_lookup_table() {
    // Succesfully close a deactived lookup table.
    let mut mollusk = setup();
    mollusk.warp_to_slot(MAX_ENTRIES as u64 + 1);

    let recipient = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let initialized_table = {
        let mut table = new_address_lookup_table(Some(authority), 0);
        table.meta.deactivation_slot = 0;
        table
    };

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    mollusk.process_and_validate_instruction(
        &close_lookup_table(lookup_table_address, authority, recipient),
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
            (recipient, AccountSharedData::default()),
        ],
        &[
            Check::success(),
            // Because lookup tables are not reassigned to the System program,
            // we can't just check for the canonical "closed" here.
            Check::account(&lookup_table_address)
                .data(&[])
                .lamports(0)
                .owner(&solana_address_lookup_table_program::id())
                .build(),
        ],
    );
}

#[test]
fn test_close_lookup_table_not_deactivated() {
    // Try to close a lookup table that hasn't first been deactivated.
    // No matter the slot, this will fail, since the lookup table must first
    // be deactived before it can be closed.
    let mollusk = setup();

    let recipient = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(Some(authority), 0);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    mollusk.process_and_validate_instruction(
        &close_lookup_table(lookup_table_address, authority, recipient),
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
            (recipient, AccountSharedData::default()),
        ],
        &[
            // The ix should fail because the table hasn't been deactivated yet
            Check::err(ProgramError::InvalidArgument),
        ],
    );
}

#[test]
fn test_close_lookup_table_deactivated() {
    // Try to close a lookup table that was deactivated, but the cooldown
    // period hasn't expired yet.
    // This should fail because the table must be deactivated in a previous
    // slot and the cooldown period must expire before it can be closed.
    let mut mollusk = setup();

    let recipient = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    // [Core BPF]: The original builtin implementation was relying on the fact
    // that the `SlotHashes` sysvar is initialized to have an entry for slot 0.
    // Program-Test does this to provide a more realistic test environment.
    // That means this test was running with the `Clock` current slot at 1.
    // In this implementation, we adapt the deactivation slot as well as the
    // current slot into tweakable test case values.
    for (deactivation_slot, current_slot) in [
        (1, 1),                 // Deactivated in the same slot
        (1, 2),                 // Deactivated one slot earlier
        (1, 40),                // Arbitrary number within cooldown (ie. 40 slot hashes 1..40).
        (1, 512),               // At the very edge of cooldown (ie. 512 slot hashes 1..512).
        (512, 512),             // Deactivated in the same slot
        (512, 512 + 1),         // Deactivated one slot earlier
        (512, 512 + 19),        // Arbitrary number within cooldown.
        (512, 512 + 511),       // At the very edge of cooldown.
        (10_000, 10_000),       // Deactivated in the same slot
        (10_000, 10_000 + 1),   // Deactivated one slot earlier
        (10_000, 10_000 + 115), // Arbitrary number within cooldown.
        (10_000, 10_000 + 511), // At the very edge of cooldown.
    ] {
        mollusk.warp_to_slot(current_slot);

        let initialized_table = {
            let mut table = new_address_lookup_table(Some(authority), 0);
            table.meta.deactivation_slot = deactivation_slot;
            table
        };

        let lookup_table_address = Pubkey::new_unique();
        let lookup_table_account = lookup_table_account(initialized_table);

        // [Core BPF]: This still holds true while using `Clock`.
        // Context sets up the slot hashes sysvar to _not_ have an entry for
        // the current slot, which is when the table was deactivated.
        //
        // When the curent slot from `Clock` is the same as the deactivation
        // slot, `LookupTableMeta::status()` should evaluate to this branch:
        //
        // ```rust
        // else if self.deactivation_slot == current_slot {
        //     LookupTableStatus::Deactivating {
        //         remaining_blocks: MAX_ENTRIES.saturating_add(1),
        //     }
        // ````
        //
        // When the deactivation slot is a prior slot, but the cooldown period
        // hasn't expired yet,`LookupTableMeta::status()` should evaluate to
        // this branch:
        //
        // ```rust
        // else if let Some(slot_position) =
        //     calculate_slot_position(&self.deactivation_slot, &current_slot)
        // {
        //     LookupTableStatus::Deactivating {
        //         remaining_blocks: MAX_ENTRIES.saturating_sub(slot_position),
        //     }
        // ````
        //
        // Because the response is not `LookupTableStatus::Deactivated`, the ix
        // should fail.
        mollusk.process_and_validate_instruction(
            &close_lookup_table(lookup_table_address, authority, recipient),
            &[
                (lookup_table_address, lookup_table_account),
                (authority, AccountSharedData::default()),
                (recipient, AccountSharedData::default()),
            ],
            &[Check::err(ProgramError::InvalidArgument)],
        );
    }
}

#[test]
fn test_close_immutable_lookup_table() {
    let mollusk = setup();

    let recipient = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(None, 0);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    mollusk.process_and_validate_instruction(
        &close_lookup_table(lookup_table_address, authority, recipient),
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
            (recipient, AccountSharedData::default()),
        ],
        &[Check::err(ProgramError::Immutable)],
    );
}

#[test]
fn test_close_lookup_table_with_wrong_authority() {
    let mollusk = setup();

    let recipient = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let wrong_authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(Some(authority), 10);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table.clone());

    mollusk.process_and_validate_instruction(
        &close_lookup_table(lookup_table_address, wrong_authority, recipient),
        &[
            (lookup_table_address, lookup_table_account),
            (wrong_authority, AccountSharedData::default()),
            (recipient, AccountSharedData::default()),
        ],
        &[Check::err(ProgramError::IncorrectAuthority)],
    );
}

#[test]
fn test_close_lookup_table_without_signing() {
    let mollusk = setup();

    let recipient = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(Some(authority), 10);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table.clone());

    let mut instruction = close_lookup_table(lookup_table_address, authority, recipient);
    instruction.accounts[1].is_signer = false;

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (lookup_table_address, lookup_table_account),
            (authority, AccountSharedData::default()),
            (recipient, AccountSharedData::default()),
        ],
        &[Check::err(ProgramError::MissingRequiredSignature)],
    );
}
