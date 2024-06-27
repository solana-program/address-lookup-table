#![cfg(feature = "test-sbf")]

use {
    common::{
        add_lookup_table_account, assert_ix_error, new_address_lookup_table,
        overwrite_slot_hashes_with_slots, setup_test_context,
    },
    solana_address_lookup_table_program::instruction::close_lookup_table,
    solana_program_test::*,
    solana_sdk::{
        clock::Clock,
        instruction::InstructionError,
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::Transaction,
    },
};

mod common;

#[tokio::test]
async fn test_close_lookup_table() {
    // Succesfully close a deactived lookup table.
    let mut context = setup_test_context().await;

    context.warp_to_slot(2).unwrap();
    overwrite_slot_hashes_with_slots(&context, &[]);

    let lookup_table_address = Pubkey::new_unique();
    let authority_keypair = Keypair::new();
    let initialized_table = {
        let mut table = new_address_lookup_table(Some(authority_keypair.pubkey()), 0);
        table.meta.deactivation_slot = 1;
        table
    };
    add_lookup_table_account(&mut context, lookup_table_address, initialized_table).await;

    let client = &mut context.banks_client;
    let payer = &context.payer;
    let recent_blockhash = context.last_blockhash;
    let transaction = Transaction::new_signed_with_payer(
        &[close_lookup_table(
            lookup_table_address,
            authority_keypair.pubkey(),
            context.payer.pubkey(),
        )],
        Some(&payer.pubkey()),
        &[payer, &authority_keypair],
        recent_blockhash,
    );

    assert!(matches!(
        client.process_transaction(transaction).await,
        Ok(())
    ));
    assert!(client
        .get_account(lookup_table_address)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_close_lookup_table_not_deactivated() {
    // Try to close a lookup table that hasn't first been deactivated.
    // No matter the slot, this will fail, since the lookup table must first
    // be deactived before it can be closed.
    let mut context = setup_test_context().await;

    let authority_keypair = Keypair::new();
    let initialized_table = new_address_lookup_table(Some(authority_keypair.pubkey()), 0);
    let lookup_table_address = Pubkey::new_unique();
    add_lookup_table_account(&mut context, lookup_table_address, initialized_table).await;

    let ix = close_lookup_table(
        lookup_table_address,
        authority_keypair.pubkey(),
        context.payer.pubkey(),
    );

    // The ix should fail because the table hasn't been deactivated yet
    assert_ix_error(
        &mut context,
        ix.clone(),
        Some(&authority_keypair),
        InstructionError::InvalidArgument,
    )
    .await;
}

#[tokio::test]
async fn test_close_lookup_table_deactivated() {
    // Try to close a lookup table that was deactivated, but the cooldown
    // period hasn't expired yet.
    // This should fail because the table must be deactivated in a previous
    // slot and the cooldown period must expire before it can be closed.
    let mut context = setup_test_context().await;

    let authority_keypair = Keypair::new();

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
        // Unfortunately, Program-Test's `warp_to_slot` causes an accounts hash
        // mismatch if you try to warp after setting an account, so we have to just
        // manipulate the `Clock` directly here.
        let mut clock = context.banks_client.get_sysvar::<Clock>().await.unwrap();
        clock.slot = current_slot;
        context.set_sysvar::<Clock>(&clock);
        overwrite_slot_hashes_with_slots(&context, &[deactivation_slot]);

        let initialized_table = {
            let mut table = new_address_lookup_table(Some(authority_keypair.pubkey()), 0);
            table.meta.deactivation_slot = deactivation_slot;
            table
        };
        let lookup_table_address = Pubkey::new_unique();
        add_lookup_table_account(&mut context, lookup_table_address, initialized_table).await;

        let ix = close_lookup_table(
            lookup_table_address,
            authority_keypair.pubkey(),
            context.payer.pubkey(),
        );

        // Because the response is not `LookupTableStatus::Deactivated`, the ix
        // should fail.
        assert_ix_error(
            &mut context,
            ix,
            Some(&authority_keypair),
            InstructionError::InvalidArgument,
        )
        .await;
    }
}

#[tokio::test]
async fn test_close_immutable_lookup_table() {
    let mut context = setup_test_context().await;

    let initialized_table = new_address_lookup_table(None, 10);
    let lookup_table_address = Pubkey::new_unique();
    add_lookup_table_account(&mut context, lookup_table_address, initialized_table).await;

    let authority = Keypair::new();
    let ix = close_lookup_table(
        lookup_table_address,
        authority.pubkey(),
        Pubkey::new_unique(),
    );

    assert_ix_error(
        &mut context,
        ix,
        Some(&authority),
        InstructionError::Immutable,
    )
    .await;
}

#[tokio::test]
async fn test_close_lookup_table_with_wrong_authority() {
    let mut context = setup_test_context().await;

    let authority = Keypair::new();
    let wrong_authority = Keypair::new();
    let initialized_table = new_address_lookup_table(Some(authority.pubkey()), 10);
    let lookup_table_address = Pubkey::new_unique();
    add_lookup_table_account(&mut context, lookup_table_address, initialized_table).await;

    let ix = close_lookup_table(
        lookup_table_address,
        wrong_authority.pubkey(),
        Pubkey::new_unique(),
    );

    assert_ix_error(
        &mut context,
        ix,
        Some(&wrong_authority),
        InstructionError::IncorrectAuthority,
    )
    .await;
}

#[tokio::test]
async fn test_close_lookup_table_without_signing() {
    let mut context = setup_test_context().await;

    let authority = Keypair::new();
    let initialized_table = new_address_lookup_table(Some(authority.pubkey()), 10);
    let lookup_table_address = Pubkey::new_unique();
    add_lookup_table_account(&mut context, lookup_table_address, initialized_table).await;

    let mut ix = close_lookup_table(
        lookup_table_address,
        authority.pubkey(),
        Pubkey::new_unique(),
    );
    ix.accounts[1].is_signer = false;

    assert_ix_error(
        &mut context,
        ix,
        None,
        InstructionError::MissingRequiredSignature,
    )
    .await;
}
