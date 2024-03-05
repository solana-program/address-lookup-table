#![allow(dead_code)]
#![cfg(feature = "test-sbf")]

use {
    scbpf_address_lookup_table::state::{AddressLookupTable, LookupTableMeta},
    solana_program_test::*,
    solana_sdk::{
        account::AccountSharedData,
        instruction::{Instruction, InstructionError},
        pubkey::Pubkey,
        signature::{Keypair, Signer},
        transaction::{Transaction, TransactionError},
    },
    std::borrow::Cow,
};

pub async fn setup_test_context() -> ProgramTestContext {
    let mut program_test = ProgramTest::default();
    program_test.prefer_bpf(false);
    program_test.add_program(
        "scbpf_address_lookup_table",
        scbpf_address_lookup_table::id(),
        processor!(scbpf_address_lookup_table::processor::process),
    );
    program_test.start_with_context().await
}

pub async fn assert_ix_error(
    context: &mut ProgramTestContext,
    ix: Instruction,
    authority_keypair: Option<&Keypair>,
    expected_err: InstructionError,
) {
    let client = &mut context.banks_client;
    let payer = &context.payer;
    let recent_blockhash = context.last_blockhash;

    let mut signers = vec![payer];
    if let Some(authority) = authority_keypair {
        signers.push(authority);
    }

    let transaction = Transaction::new_signed_with_payer(
        &[ix],
        Some(&payer.pubkey()),
        &signers,
        recent_blockhash,
    );

    assert_eq!(
        client
            .process_transaction(transaction)
            .await
            .unwrap_err()
            .unwrap(),
        TransactionError::InstructionError(0, expected_err),
    );
}

pub fn new_address_lookup_table(
    authority: Option<Pubkey>,
    num_addresses: usize,
) -> AddressLookupTable<'static> {
    let mut addresses = Vec::with_capacity(num_addresses);
    addresses.resize_with(num_addresses, Pubkey::new_unique);
    AddressLookupTable {
        meta: LookupTableMeta {
            authority,
            ..LookupTableMeta::default()
        },
        addresses: Cow::Owned(addresses),
    }
}

pub async fn add_lookup_table_account(
    context: &mut ProgramTestContext,
    account_address: Pubkey,
    address_lookup_table: AddressLookupTable<'static>,
) -> AccountSharedData {
    let data = address_lookup_table.serialize_for_tests().unwrap();
    let rent = context.banks_client.get_rent().await.unwrap();
    let rent_exempt_balance = rent.minimum_balance(data.len());

    let mut account = AccountSharedData::new(
        rent_exempt_balance,
        data.len(),
        &scbpf_address_lookup_table::id(),
    );
    account.set_data_from_slice(&data);
    context.set_account(&account_address, &account);

    account
}
