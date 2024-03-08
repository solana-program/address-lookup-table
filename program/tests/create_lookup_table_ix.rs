#![cfg(feature = "test-sbf")]

use {
    common::{assert_ix_error, setup_test_context},
    scbpf_address_lookup_table::{
        instruction::create_lookup_table,
        state::{AddressLookupTable, LOOKUP_TABLE_META_SIZE},
    },
    solana_program_test::*,
    solana_sdk::{
        clock::Slot, instruction::InstructionError, pubkey::Pubkey, rent::Rent, signature::Signer,
        transaction::Transaction,
    },
};

mod common;

// [Core BPF]: Tests that assert proper authority checks have been removed,
// since feature "FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap"
// (relax_authority_signer_check_for_lookup_table_creation) has been activated
// on all clusters.

#[tokio::test]
async fn test_create_lookup_table_idempotent() {
    let mut context = setup_test_context().await;

    let test_recent_slot = 123;
    // [Core BPF]: Warping to slot instead of overwriting `SlotHashes`.
    context.warp_to_slot(test_recent_slot + 1).unwrap();

    let client = &mut context.banks_client;
    let payer = &context.payer;
    let recent_blockhash = context.last_blockhash;
    let authority_address = Pubkey::new_unique();
    let (create_lookup_table_ix, lookup_table_address) =
        create_lookup_table(authority_address, payer.pubkey(), test_recent_slot);

    // First create should succeed
    {
        let transaction = Transaction::new_signed_with_payer(
            &[create_lookup_table_ix.clone()],
            Some(&payer.pubkey()),
            &[payer],
            recent_blockhash,
        );

        assert!(matches!(
            client.process_transaction(transaction).await,
            Ok(())
        ));
        let lookup_table_account = client
            .get_account(lookup_table_address)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(lookup_table_account.owner, scbpf_address_lookup_table::id());
        assert_eq!(lookup_table_account.data.len(), LOOKUP_TABLE_META_SIZE);
        assert_eq!(
            lookup_table_account.lamports,
            Rent::default().minimum_balance(LOOKUP_TABLE_META_SIZE)
        );
        let lookup_table = AddressLookupTable::deserialize(&lookup_table_account.data).unwrap();
        assert_eq!(lookup_table.meta.deactivation_slot, Slot::MAX);
        assert_eq!(lookup_table.meta.authority, Some(authority_address));
        assert_eq!(lookup_table.meta.last_extended_slot, 0);
        assert_eq!(lookup_table.meta.last_extended_slot_start_index, 0);
        assert_eq!(lookup_table.addresses.len(), 0);
    }

    // Second create should succeed too
    {
        let recent_blockhash = client
            .get_new_latest_blockhash(&recent_blockhash)
            .await
            .unwrap();
        let transaction = Transaction::new_signed_with_payer(
            &[create_lookup_table_ix],
            Some(&payer.pubkey()),
            &[payer],
            recent_blockhash,
        );

        assert!(matches!(
            client.process_transaction(transaction).await,
            Ok(())
        ));
    }
}

#[tokio::test]
async fn test_create_lookup_table_not_recent_slot() {
    let mut context = setup_test_context().await;
    let payer = &context.payer;
    let authority_address = Pubkey::new_unique();

    let ix = create_lookup_table(authority_address, payer.pubkey(), Slot::MAX).0;

    assert_ix_error(
        &mut context,
        ix,
        None,
        InstructionError::InvalidInstructionData,
    )
    .await;
}

#[tokio::test]
async fn test_create_lookup_table_pda_mismatch() {
    let mut context = setup_test_context().await;

    let test_recent_slot = 123;
    // [Core BPF]: Warping to slot instead of overwriting `SlotHashes`.
    context.warp_to_slot(test_recent_slot + 1).unwrap();

    let payer = &context.payer;
    let authority_address = Pubkey::new_unique();

    let mut ix = create_lookup_table(authority_address, payer.pubkey(), test_recent_slot).0;
    ix.accounts[0].pubkey = Pubkey::new_unique();

    assert_ix_error(&mut context, ix, None, InstructionError::InvalidArgument).await;
}
