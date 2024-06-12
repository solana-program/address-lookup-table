use {
    super::TEST_CLOCK_SLOT,
    mollusk_bencher::Bench,
    solana_address_lookup_table_program::{
        instruction::{
            close_lookup_table as close_lookup_table_ix,
            create_lookup_table as create_lookup_table_ix,
            deactivate_lookup_table as deactivate_lookup_table_ix,
            extend_lookup_table as extend_lookup_table_ix,
            freeze_lookup_table as freeze_lookup_table_ix,
        },
        state::{AddressLookupTable, LookupTableMeta},
    },
    solana_sdk::{account::AccountSharedData, pubkey::Pubkey, rent::Rent, system_program},
    std::borrow::Cow,
};

fn lookup_table_account(
    authority: &Pubkey,
    num_keys: usize,
    deactivated: bool,
) -> AccountSharedData {
    let state = {
        let mut addresses = Vec::with_capacity(num_keys);
        addresses.resize_with(num_keys, Pubkey::new_unique);
        AddressLookupTable {
            meta: LookupTableMeta {
                authority: Some(*authority),
                deactivation_slot: if deactivated { 1 } else { u64::MAX },
                ..LookupTableMeta::default()
            },
            addresses: Cow::Owned(addresses),
        }
    };
    let data = state.serialize_for_tests().unwrap();
    let data_len = data.len();
    let lamports = Rent::default().minimum_balance(data_len);
    let mut account = AccountSharedData::new(
        lamports,
        data_len,
        &solana_address_lookup_table_program::id(),
    );
    account.set_data_from_slice(&data);
    account
}

pub fn create_lookup_table() -> Bench {
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();

    let (instruction, lookup_table) = create_lookup_table_ix(authority, payer, TEST_CLOCK_SLOT - 1);

    (
        "create_lookup_table".to_string(),
        instruction,
        vec![
            (lookup_table, AccountSharedData::default()),
            (authority, AccountSharedData::default()),
            (
                payer,
                AccountSharedData::new(100_000_000_000, 0, &system_program::id()),
            ),
            (
                solana_sdk::system_program::id(),
                mollusk::programs::system_program_account(&Rent::default()),
            ),
        ],
    )
}

pub fn extend_lookup_table(from: usize, to: usize) -> Bench {
    let lookup_table = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();

    let new_addresses = (from..to).map(|_| Pubkey::new_unique()).collect::<Vec<_>>();

    (
        format!("extend_lookup_table_from_{}_to_{}", from, to),
        extend_lookup_table_ix(lookup_table, authority, Some(payer), new_addresses),
        vec![
            (lookup_table, lookup_table_account(&authority, from, false)),
            (authority, AccountSharedData::default()),
            (
                payer,
                AccountSharedData::new(100_000_000_000, 0, &system_program::id()),
            ),
            (
                solana_sdk::system_program::id(),
                mollusk::programs::system_program_account(&Rent::default()),
            ),
        ],
    )
}

pub fn freeze_lookup_table() -> Bench {
    let lookup_table = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    (
        "freeze_lookup_table".to_string(),
        freeze_lookup_table_ix(lookup_table, authority),
        vec![
            (lookup_table, lookup_table_account(&authority, 1, false)),
            (authority, AccountSharedData::default()),
        ],
    )
}

pub fn deactivate_lookup_table() -> Bench {
    let lookup_table = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    (
        "deactivate_lookup_table".to_string(),
        deactivate_lookup_table_ix(lookup_table, authority),
        vec![
            (lookup_table, lookup_table_account(&authority, 1, false)),
            (authority, AccountSharedData::default()),
        ],
    )
}

pub fn close_lookup_table() -> Bench {
    let lookup_table = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    (
        "close_lookup_table".to_string(),
        close_lookup_table_ix(lookup_table, authority, recipient),
        vec![
            (lookup_table, lookup_table_account(&authority, 1, true)),
            (authority, AccountSharedData::default()),
            (recipient, AccountSharedData::default()),
        ],
    )
}
