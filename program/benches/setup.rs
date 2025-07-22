use {
    mollusk_svm::program::keyed_account_for_system_program,
    mollusk_svm_bencher::Bench,
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
    solana_sdk::{account::Account, instruction::Instruction, pubkey::Pubkey, rent::Rent},
    solana_sdk_ids::system_program,
    std::borrow::Cow,
};

pub const TEST_CLOCK_SLOT: u64 = 100_000;

/// Helper struct to convert to a `Bench`.
pub struct BenchContext {
    label: String,
    instruction: Instruction,
    accounts: Vec<(Pubkey, Account)>,
}

impl BenchContext {
    /// Convert to a `Bench`.
    pub fn bench(&self) -> Bench {
        (self.label.as_str(), &self.instruction, &self.accounts)
    }
}

fn lookup_table_account(authority: &Pubkey, num_keys: usize, deactivated: bool) -> Account {
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
    let mut account = Account::new(
        lamports,
        data_len,
        &solana_address_lookup_table_program::id(),
    );
    account.data = data;
    account
}

pub fn create_lookup_table() -> BenchContext {
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();

    let (instruction, lookup_table) = create_lookup_table_ix(authority, payer, TEST_CLOCK_SLOT - 1);

    let accounts = vec![
        (lookup_table, Account::default()),
        (authority, Account::default()),
        (
            payer,
            Account::new(100_000_000_000, 0, &system_program::id()),
        ),
        keyed_account_for_system_program(),
    ];

    BenchContext {
        label: "create_lookup_table".to_string(),
        instruction,
        accounts,
    }
}

pub fn extend_lookup_table(from: usize, to: usize) -> BenchContext {
    let lookup_table = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let payer = Pubkey::new_unique();

    let new_addresses = (from..to).map(|_| Pubkey::new_unique()).collect::<Vec<_>>();

    let instruction = extend_lookup_table_ix(lookup_table, authority, Some(payer), new_addresses);

    let accounts = vec![
        (lookup_table, lookup_table_account(&authority, from, false)),
        (authority, Account::default()),
        (
            payer,
            Account::new(100_000_000_000, 0, &system_program::id()),
        ),
        keyed_account_for_system_program(),
    ];

    BenchContext {
        label: format!("extend_lookup_table_from_{}_to_{}", from, to),
        instruction,
        accounts,
    }
}

pub fn freeze_lookup_table() -> BenchContext {
    let lookup_table = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let instruction = freeze_lookup_table_ix(lookup_table, authority);

    let accounts = vec![
        (lookup_table, lookup_table_account(&authority, 1, false)),
        (authority, Account::default()),
    ];

    BenchContext {
        label: "freeze_lookup_table".to_string(),
        instruction,
        accounts,
    }
}

pub fn deactivate_lookup_table() -> BenchContext {
    let lookup_table = Pubkey::new_unique();
    let authority = Pubkey::new_unique();

    let instruction = deactivate_lookup_table_ix(lookup_table, authority);

    let accounts = vec![
        (lookup_table, lookup_table_account(&authority, 1, false)),
        (authority, Account::default()),
    ];

    BenchContext {
        label: "deactivate_lookup_table".to_string(),
        instruction,
        accounts,
    }
}

pub fn close_lookup_table() -> BenchContext {
    let lookup_table = Pubkey::new_unique();
    let authority = Pubkey::new_unique();
    let recipient = Pubkey::new_unique();

    let instruction = close_lookup_table_ix(lookup_table, authority, recipient);

    let accounts = vec![
        (lookup_table, lookup_table_account(&authority, 1, true)),
        (authority, Account::default()),
        (recipient, Account::default()),
    ];

    BenchContext {
        label: "close_lookup_table".to_string(),
        instruction,
        accounts,
    }
}
