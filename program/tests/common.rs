#![allow(dead_code)]
use {
    mollusk_svm::Mollusk,
    solana_address_lookup_table_program::state::{AddressLookupTable, LookupTableMeta},
    solana_sdk::{account::Account, pubkey::Pubkey, rent::Rent},
    std::borrow::Cow,
};

pub fn setup() -> Mollusk {
    Mollusk::new(
        &solana_address_lookup_table_program::id(),
        "solana_address_lookup_table_program",
    )
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

pub fn lookup_table_account(address_lookup_table: AddressLookupTable<'static>) -> Account {
    let data = address_lookup_table.serialize_for_tests().unwrap();
    let rent_exempt_balance = Rent::default().minimum_balance(data.len());
    let mut account = Account::new(
        rent_exempt_balance,
        data.len(),
        &solana_address_lookup_table_program::id(),
    );
    account.data = data;
    account
}
