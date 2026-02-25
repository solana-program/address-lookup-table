mod common;

use {
    common::{lookup_table_account, new_address_lookup_table, setup},
    mollusk_svm::{
        program::keyed_account_for_system_program,
        result::{Check, ProgramResult},
        Mollusk,
    },
    solana_account::{Account, ReadableAccount, WritableAccount},
    solana_address_lookup_table_program::{
        error::AddressLookupTableError,
        instruction::extend_lookup_table,
        state::{AddressLookupTable, LookupTableMeta},
    },
    solana_instruction::Instruction,
    solana_program_error::ProgramError,
    solana_pubkey::{Pubkey, PUBKEY_BYTES},
    solana_sdk_ids::system_program,
    std::{borrow::Cow, result::Result},
};

struct ExpectedTableAccount {
    lamports: u64,
    data_len: usize,
    state: AddressLookupTable<'static>,
}

struct TestCase {
    lookup_table_address: Pubkey,
    instruction: Instruction,
    accounts: Vec<(Pubkey, Account)>,
    expected_result: Result<ExpectedTableAccount, ProgramError>,
}

fn run_test_case(mollusk: &Mollusk, test_case: TestCase) {
    let result = mollusk.process_instruction(&test_case.instruction, &test_case.accounts);

    match test_case.expected_result {
        Ok(expected_account) => {
            assert!(matches!(result.program_result, ProgramResult::Success));

            let table_account = result.get_account(&test_case.lookup_table_address).unwrap();
            let lookup_table = AddressLookupTable::deserialize(table_account.data()).unwrap();
            assert_eq!(lookup_table, expected_account.state);
            assert_eq!(table_account.lamports(), expected_account.lamports);
            assert_eq!(table_account.data().len(), expected_account.data_len);
        }
        Err(expected_err) => {
            assert_eq!(result.program_result, ProgramResult::Failure(expected_err));
        }
    }
}

#[test]
fn test_extend_lookup_table() {
    let mut mollusk = setup();
    mollusk.warp_to_slot(1); // Mollusk starts at slot 0, where program-test would start at 1.

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let current_bank_slot = 1;
    let rent = mollusk.sysvars.rent.clone();

    for extend_same_slot in [true, false] {
        for (num_existing_addresses, num_new_addresses, expected_result) in [
            (0, 0, Err(ProgramError::InvalidInstructionData)),
            (0, 1, Ok(())),
            (0, 10, Ok(())),
            (0, 38, Ok(())), // Max new addresses allowed by `limited_deserialize`
            (0, 39, Err(ProgramError::InvalidInstructionData)),
            (1, 1, Ok(())),
            (1, 10, Ok(())),
            (218, 38, Ok(())), // 38 less than maximum, 38 brings it to the maximum
            (219, 38, Err(ProgramError::InvalidInstructionData)),
            (246, 10, Ok(())),
            (255, 1, Ok(())), // One less than maximum, 1 brings it to the maximum
            (255, 2, Err(ProgramError::InvalidInstructionData)),
            (256, 1, Err(ProgramError::InvalidArgument)),
        ] {
            let mut lookup_table =
                new_address_lookup_table(Some(authority), num_existing_addresses);
            if extend_same_slot {
                lookup_table.meta.last_extended_slot = current_bank_slot;
            }

            let lookup_table_address = Pubkey::new_unique();
            let lookup_table_account = lookup_table_account(lookup_table.clone());

            let mut new_addresses = Vec::with_capacity(num_new_addresses);
            new_addresses.resize_with(num_new_addresses, Pubkey::new_unique);

            let instruction = extend_lookup_table(
                lookup_table_address,
                authority,
                Some(payer),
                new_addresses.clone(),
            );

            let accounts = vec![
                (lookup_table_address, lookup_table_account.clone()),
                (authority, Account::default()),
                (payer, Account::new(100_000_000, 0, &system_program::id())),
                keyed_account_for_system_program(),
            ];

            let mut expected_addresses: Vec<Pubkey> = lookup_table.addresses.to_vec();
            expected_addresses.extend(new_addresses);

            let expected_result = expected_result.map(|_| {
                let expected_data_len =
                    lookup_table_account.data().len() + num_new_addresses * PUBKEY_BYTES;
                let expected_lamports = rent.minimum_balance(expected_data_len);
                let expected_lookup_table = AddressLookupTable {
                    meta: LookupTableMeta {
                        last_extended_slot: current_bank_slot,
                        last_extended_slot_start_index: if extend_same_slot {
                            0u8
                        } else {
                            num_existing_addresses as u8
                        },
                        deactivation_slot: lookup_table.meta.deactivation_slot,
                        authority: lookup_table.meta.authority,
                        _padding: 0u16,
                    },
                    addresses: Cow::Owned(expected_addresses),
                };
                ExpectedTableAccount {
                    lamports: expected_lamports,
                    data_len: expected_data_len,
                    state: expected_lookup_table,
                }
            });

            let test_case = TestCase {
                lookup_table_address,
                instruction,
                accounts,
                expected_result,
            };

            run_test_case(&mollusk, test_case);
        }
    }
}

#[test]
fn test_extend_lookup_table_with_wrong_authority() {
    let mollusk = setup();

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let wrong_authority = Pubkey::new_unique();

    let initialized_table = new_address_lookup_table(Some(authority), 0);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    let new_addresses = vec![Pubkey::new_unique()];
    let instruction = extend_lookup_table(
        lookup_table_address,
        wrong_authority,
        Some(payer),
        new_addresses,
    );

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (lookup_table_address, lookup_table_account),
            (wrong_authority, Account::default()),
            (payer, Account::new(100_000_000, 0, &system_program::id())),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::IncorrectAuthority)],
    );
}

#[test]
fn test_extend_lookup_table_without_signing() {
    let mollusk = setup();

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(Some(authority), 10);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    let new_addresses = vec![Pubkey::new_unique()];
    let mut instruction =
        extend_lookup_table(lookup_table_address, authority, Some(payer), new_addresses);
    instruction.accounts[1].is_signer = false;

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (lookup_table_address, lookup_table_account),
            (authority, Account::default()),
            (payer, Account::new(100_000_000, 0, &system_program::id())),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::MissingRequiredSignature)],
    );
}

#[test]
fn test_extend_deactivated_lookup_table() {
    let mollusk = setup();

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let initialized_table = {
        let mut table = new_address_lookup_table(Some(authority), 0);
        table.meta.deactivation_slot = 0;
        table
    };

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    let new_addresses = vec![Pubkey::new_unique()];
    let instruction =
        extend_lookup_table(lookup_table_address, authority, Some(payer), new_addresses);

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (lookup_table_address, lookup_table_account),
            (authority, Account::default()),
            (payer, Account::new(100_000_000, 0, &system_program::id())),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::InvalidArgument)],
    );
}

#[test]
fn test_extend_immutable_lookup_table() {
    let mollusk = setup();

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(None, 1);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    let new_addresses = vec![Pubkey::new_unique()];
    let instruction =
        extend_lookup_table(lookup_table_address, authority, Some(payer), new_addresses);

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (lookup_table_address, lookup_table_account),
            (authority, Account::default()),
            (payer, Account::new(100_000_000, 0, &system_program::id())),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::Immutable)],
    );
}

#[test]
fn test_extend_lookup_table_without_payer() {
    let mollusk = setup();

    let authority = Pubkey::new_unique();
    let initialized_table = new_address_lookup_table(Some(authority), 0);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    let new_addresses = vec![Pubkey::new_unique()];
    let instruction = extend_lookup_table(lookup_table_address, authority, None, new_addresses);

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (lookup_table_address, lookup_table_account),
            (authority, Account::default()),
        ],
        &[Check::err(ProgramError::NotEnoughAccountKeys)],
    );
}

#[test]
fn test_extend_prepaid_lookup_table_without_payer() {
    let mollusk = setup();

    let authority = Pubkey::new_unique();
    let lookup_table_address = Pubkey::new_unique();

    let (lookup_table_account, expected_state) = {
        // initialize lookup table
        let empty_lookup_table = new_address_lookup_table(Some(authority), 0);
        let mut lookup_table_account = lookup_table_account(empty_lookup_table);

        // calculate required rent exempt balance for adding one address
        let mut temp_lookup_table = new_address_lookup_table(Some(authority), 1);
        let data = temp_lookup_table.clone().serialize_for_tests().unwrap();
        let rent_exempt_balance = mollusk.sysvars.rent.minimum_balance(data.len());

        // prepay for one address
        lookup_table_account.set_lamports(rent_exempt_balance);

        // test will extend table in the current bank's slot
        temp_lookup_table.meta.last_extended_slot = mollusk.sysvars.clock.slot;

        (
            lookup_table_account,
            ExpectedTableAccount {
                lamports: rent_exempt_balance,
                data_len: data.len(),
                state: temp_lookup_table,
            },
        )
    };

    let new_addresses = expected_state.state.addresses.to_vec();
    let instruction = extend_lookup_table(lookup_table_address, authority, None, new_addresses);

    let accounts = vec![
        (lookup_table_address, lookup_table_account),
        (authority, Account::default()),
    ];

    run_test_case(
        &mollusk,
        TestCase {
            lookup_table_address,
            instruction,
            accounts,
            expected_result: Ok(expected_state),
        },
    );
}

// Backwards compatibility test case.
#[test]
fn test_extend_readonly() {
    let mollusk = setup();

    let payer = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let initialized_table = new_address_lookup_table(Some(authority), 0);

    let lookup_table_address = Pubkey::new_unique();
    let lookup_table_account = lookup_table_account(initialized_table);

    let new_addresses = vec![Pubkey::new_unique()];
    let mut instruction =
        extend_lookup_table(lookup_table_address, authority, Some(payer), new_addresses);

    // Make the lookup table account read-only.
    instruction.accounts[0].is_writable = false;

    mollusk.process_and_validate_instruction(
        &instruction,
        &[
            (lookup_table_address, lookup_table_account),
            (authority, Account::default()),
            (payer, Account::new(100_000_000, 0, &system_program::id())),
            keyed_account_for_system_program(),
        ],
        &[Check::err(ProgramError::Custom(
            AddressLookupTableError::ReadonlyDataModified as u32,
        ))],
    );
}
