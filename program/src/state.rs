use {
    crate::error::AddressLookupError,
    serde::{Deserialize, Serialize},
    solana_frozen_abi_macro::{AbiEnumVisitor, AbiExample},
    solana_program::{
        clock::Slot, program_error::ProgramError, pubkey::Pubkey, slot_hashes::MAX_ENTRIES,
    },
    std::borrow::Cow,
};

/// The maximum number of addresses that a lookup table can hold
pub const LOOKUP_TABLE_MAX_ADDRESSES: usize = 256;

/// The serialized size of lookup table metadata
pub const LOOKUP_TABLE_META_SIZE: usize = 56;

// [Core BPF]: Newly-implemented logic for calculating slot position relative
// to the current slot on the `Clock`.
// In the original implementation, `slot_hashes.position()` can return
// `Some(position)` where `position` is in the range `0..511`.
// Position `0` means `MAX_ENTRIES - 0 = 512` blocks remaining.
// Position `511` means `MAX_ENTRIES - 511 = 1` block remaining.
// To account for that range, considering the current slot would not be present
// in the `SlotHashes` sysvar, we need to first subtract `1` from the current
// slot, and then subtract the target slot from the result.
fn calculate_slot_position(target_slot: &Slot, current_slot: &Slot) -> Option<usize> {
    let position = current_slot.saturating_sub(*target_slot);

    if position >= (MAX_ENTRIES as u64) {
        return None;
    }
    Some(position as usize)
}

/// Activation status of a lookup table
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum LookupTableStatus {
    Activated,
    Deactivating { remaining_blocks: usize },
    Deactivated,
}

/// Address lookup table metadata
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, AbiExample)]
pub struct LookupTableMeta {
    // [Core BPF]: TODO: `Clock` instead of `SlotHashes`.
    /// Lookup tables cannot be closed until the deactivation slot is
    /// no longer "recent" (not accessible in the `SlotHashes` sysvar).
    pub deactivation_slot: Slot,
    /// The slot that the table was last extended. Address tables may
    /// only be used to lookup addresses that were extended before
    /// the current bank's slot.
    pub last_extended_slot: Slot,
    /// The start index where the table was last extended from during
    /// the `last_extended_slot`.
    pub last_extended_slot_start_index: u8,
    /// Authority address which must sign for each modification.
    pub authority: Option<Pubkey>,
    // Padding to keep addresses 8-byte aligned
    pub _padding: u16,
    // Raw list of addresses follows this serialized structure in
    // the account's data, starting from `LOOKUP_TABLE_META_SIZE`.
}

impl Default for LookupTableMeta {
    fn default() -> Self {
        Self {
            deactivation_slot: Slot::MAX,
            last_extended_slot: 0,
            last_extended_slot_start_index: 0,
            authority: None,
            _padding: 0,
        }
    }
}

impl LookupTableMeta {
    pub fn new(authority: Pubkey) -> Self {
        LookupTableMeta {
            authority: Some(authority),
            ..LookupTableMeta::default()
        }
    }

    /// Returns whether the table is considered active for address lookups
    pub fn is_active(&self, current_slot: Slot) -> bool {
        match self.status(current_slot) {
            LookupTableStatus::Activated => true,
            LookupTableStatus::Deactivating { .. } => true,
            LookupTableStatus::Deactivated => false,
        }
    }

    // [Core BPF]: This function has been modified from its legacy built-in
    // counterpart to no longer use the `SlotHashes` sysvar, since it is not
    // available for BPF programs. Instead, it uses the `current_slot`
    // parameter to calculate the table's status.
    // This will no longer consider the case where a slot has been skipped
    // and no block was produced.
    // If it's imperative to ensure we are only considering slots where blocks
    // were created, then we'll need to revisit this function, and possibly
    // provide the `SlotHashes` account so we can reliably check slot hashes.
    /// Return the current status of the lookup table
    pub fn status(&self, current_slot: Slot) -> LookupTableStatus {
        if self.deactivation_slot == Slot::MAX {
            LookupTableStatus::Activated
        } else if self.deactivation_slot == current_slot {
            LookupTableStatus::Deactivating {
                remaining_blocks: MAX_ENTRIES,
            }
        } else if let Some(slot_position) =
            calculate_slot_position(&self.deactivation_slot, &current_slot)
        {
            // [Core BPF]: TODO: `Clock` instead of `SlotHashes`.
            // Deactivation requires a cool-down period to give in-flight transactions
            // enough time to land and to remove indeterminism caused by transactions
            // loading addresses in the same slot when a table is closed. The
            // cool-down period is equivalent to the amount of time it takes for
            // a slot to be removed from the slot hash list.
            //
            // By using the slot hash to enforce the cool-down, there is a side effect
            // of not allowing lookup tables to be recreated at the same derived address
            // because tables must be created at an address derived from a recent slot.
            LookupTableStatus::Deactivating {
                remaining_blocks: MAX_ENTRIES.saturating_sub(slot_position),
            }
        } else {
            LookupTableStatus::Deactivated
        }
    }
}

/// Program account states
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone, AbiExample, AbiEnumVisitor)]
#[allow(clippy::large_enum_variant)]
pub enum ProgramState {
    /// Account is not initialized.
    Uninitialized,
    /// Initialized `LookupTable` account.
    LookupTable(LookupTableMeta),
}

impl ProgramState {
    // [Core BPF]: This is a new function that was not present in the legacy
    // built-in implementation.
    /// Serialize a new lookup table into uninitialized account data.
    pub fn serialize_new_lookup_table(
        data: &mut [u8],
        authority_key: &Pubkey,
    ) -> Result<(), ProgramError> {
        let lookup_table = ProgramState::LookupTable(LookupTableMeta::new(*authority_key));
        // [Core BPF]: The original builtin implementation mapped `bincode`
        // serialization errors to `InstructionError::GenericError`, but this
        // error is deprecated. The error code for failed serialization has
        // changed.
        let serialized_size = bincode::serialized_size(&lookup_table)
            .map_err(|_| ProgramError::InvalidAccountData)?;
        // [Core BPF]: Although this check may seem unnecessary, since
        // `bincode::serialize_into` will throw
        // `ProgramError::InvalidAccountData` if the data is not large enough,
        // `AccountDataTooSmall` is the error thrown by the original built-in
        // through `BorrowedAccount::set_state`, which employs this check.
        // Note the original implementation did not check for data that was
        // too large, nor did it check to make sure the data was all `0`.
        if serialized_size > data.len() as u64 {
            return Err(ProgramError::AccountDataTooSmall);
        }
        bincode::serialize_into(data, &lookup_table).map_err(|_| ProgramError::InvalidAccountData)
    }
}

#[derive(Debug, PartialEq, Eq, Clone, AbiExample)]
pub struct AddressLookupTable<'a> {
    pub meta: LookupTableMeta,
    pub addresses: Cow<'a, [Pubkey]>,
}

impl<'a> AddressLookupTable<'a> {
    /// Serialize an address table's updated meta data and zero
    /// any leftover bytes.
    pub fn overwrite_meta_data(
        data: &mut [u8],
        lookup_table_meta: LookupTableMeta,
    ) -> Result<(), ProgramError> {
        let meta_data = data
            .get_mut(0..LOOKUP_TABLE_META_SIZE)
            .ok_or(ProgramError::InvalidAccountData)?;
        meta_data.fill(0);
        bincode::serialize_into(meta_data, &ProgramState::LookupTable(lookup_table_meta))
            // [Core BPF]: The original builtin implementation mapped `bincode`
            // serialization errors to `InstructionError::GenericError`, but this
            // error is deprecated. The error code for failed serialization has
            // changed.
            .map_err(|_| ProgramError::InvalidAccountData)
    }

    /// Get the length of addresses that are active for lookups
    pub fn get_active_addresses_len(
        &self,
        current_slot: Slot,
    ) -> Result<usize, AddressLookupError> {
        if !self.meta.is_active(current_slot) {
            // Once a lookup table is no longer active, it can be closed
            // at any point, so returning a specific error for deactivated
            // lookup tables could result in a race condition.
            return Err(AddressLookupError::LookupTableAccountNotFound);
        }

        // If the address table was extended in the same slot in which it is used
        // to lookup addresses for another transaction, the recently extended
        // addresses are not considered active and won't be accessible.
        let active_addresses_len = if current_slot > self.meta.last_extended_slot {
            self.addresses.len()
        } else {
            self.meta.last_extended_slot_start_index as usize
        };

        Ok(active_addresses_len)
    }

    /// Lookup addresses for provided table indexes. Since lookups are performed
    /// on tables which are not read-locked, this implementation needs to be
    /// careful about resolving addresses consistently.
    pub fn lookup(
        &self,
        current_slot: Slot,
        indexes: &[u8],
    ) -> Result<Vec<Pubkey>, AddressLookupError> {
        let active_addresses_len = self.get_active_addresses_len(current_slot)?;
        let active_addresses = &self.addresses[0..active_addresses_len];
        indexes
            .iter()
            .map(|idx| active_addresses.get(*idx as usize).cloned())
            .collect::<Option<_>>()
            .ok_or(AddressLookupError::InvalidLookupIndex)
    }

    /// Serialize an address table including its addresses
    pub fn serialize_for_tests(self) -> Result<Vec<u8>, ProgramError> {
        let mut data = vec![0; LOOKUP_TABLE_META_SIZE];
        Self::overwrite_meta_data(&mut data, self.meta)?;
        self.addresses.iter().for_each(|address| {
            data.extend_from_slice(address.as_ref());
        });
        Ok(data)
    }

    // [Core BPF]: This is a new function that was not present in the legacy
    // built-in implementation.
    /// Mutably deserialize addresses from a lookup table's data. This function
    /// accepts an index in the list of addresses to start deserializing from.
    pub fn deserialize_addresses_from_index_mut(
        data: &mut [u8],
        index: u8,
    ) -> Result<&mut [Pubkey], ProgramError> {
        let offset = LOOKUP_TABLE_META_SIZE
            .checked_add((index as usize).saturating_mul(std::mem::size_of::<Pubkey>()))
            .ok_or(ProgramError::ArithmeticOverflow)?;
        if offset >= data.len() {
            return Err(ProgramError::InvalidArgument);
        }
        bytemuck::try_cast_slice_mut(&mut data[offset..]).map_err(|_| {
            // Should be impossible because raw address data
            // should be aligned and sized in multiples of 32 bytes
            ProgramError::InvalidAccountData
        })
    }

    /// Efficiently deserialize an address table without allocating
    /// for stored addresses.
    pub fn deserialize(data: &'a [u8]) -> Result<AddressLookupTable<'a>, ProgramError> {
        let program_state: ProgramState =
            bincode::deserialize(data).map_err(|_| ProgramError::InvalidAccountData)?;

        let meta = match program_state {
            ProgramState::LookupTable(meta) => Ok(meta),
            ProgramState::Uninitialized => Err(ProgramError::UninitializedAccount),
        }?;

        let raw_addresses_data = data.get(LOOKUP_TABLE_META_SIZE..).ok_or({
            // Should be impossible because table accounts must
            // always be LOOKUP_TABLE_META_SIZE in length
            ProgramError::InvalidAccountData
        })?;
        let addresses: &[Pubkey] = bytemuck::try_cast_slice(raw_addresses_data).map_err(|_| {
            // Should be impossible because raw address data
            // should be aligned and sized in multiples of 32 bytes
            ProgramError::InvalidAccountData
        })?;

        Ok(Self {
            meta,
            addresses: Cow::Borrowed(addresses),
        })
    }
}

#[cfg(test)]
mod tests {
    use {super::*, test_case::test_case};

    impl AddressLookupTable<'_> {
        fn new_for_tests(meta: LookupTableMeta, num_addresses: usize) -> Self {
            let mut addresses = Vec::with_capacity(num_addresses);
            addresses.resize_with(num_addresses, Pubkey::new_unique);
            AddressLookupTable {
                meta,
                addresses: Cow::Owned(addresses),
            }
        }
    }

    impl LookupTableMeta {
        fn new_for_tests() -> Self {
            Self {
                authority: Some(Pubkey::new_unique()),
                ..LookupTableMeta::default()
            }
        }
    }

    #[test]
    fn test_lookup_table_meta_size() {
        let lookup_table = ProgramState::LookupTable(LookupTableMeta::new_for_tests());
        let meta_size = bincode::serialized_size(&lookup_table).unwrap();
        assert!(meta_size as usize <= LOOKUP_TABLE_META_SIZE);
        assert_eq!(meta_size as usize, 56);

        let lookup_table = ProgramState::LookupTable(LookupTableMeta::default());
        let meta_size = bincode::serialized_size(&lookup_table).unwrap();
        assert!(meta_size as usize <= LOOKUP_TABLE_META_SIZE);
        assert_eq!(meta_size as usize, 24);
    }

    // [Core BPF]: This test has been rewritten to test the new
    // `calculate_slot_position` status functionality based on `Clock` rather
    // than `SlotHashes`.
    // Written intentionally verbose.
    // rustfmt-ignore
    #[test_case(
        Slot::MAX,
        0,
        LookupTableStatus::Activated;
        "activated"
    )]
    #[test_case(
        Slot::MAX,
        511,
        LookupTableStatus::Activated;
        "activated_current_slot_doesnt_matter"
    )]
    // Here we hit branch `self.deactivation_slot == current_slot`.
    #[test_case(
        0,
        0,
        LookupTableStatus::Deactivating { remaining_blocks: MAX_ENTRIES }; // 512
        "d0::deactivated_in_current_slot"
    )]
    // Here `calculate_slot_position` returns `Some(0)`.
    #[test_case(
        0,
        0 + 1,
        LookupTableStatus::Deactivating { remaining_blocks: MAX_ENTRIES - 1 }; // 511
        "d0::deactivated_one_slot_ago"
    )]
    // Here `calculate_slot_position` returns `None`.
    #[test_case(
        0,
        0 + MAX_ENTRIES as u64,
        LookupTableStatus::Deactivated;
        "d0::cooldown_expired"
    )]
    // Here we hit branch `self.deactivation_slot == current_slot`.
    #[test_case(
        1,
        1,
        LookupTableStatus::Deactivating { remaining_blocks: MAX_ENTRIES }; // 512
        "d1::deactivated_in_current_slot"
    )]
    // Here `calculate_slot_position` returns `Some(0)`.
    #[test_case(
        1,
        1 + 1,
        LookupTableStatus::Deactivating { remaining_blocks: MAX_ENTRIES - 1 }; // 511
        "d1::deactivated_one_slot_ago"
    )]
    // Here `calculate_slot_position` returns `None`.
    #[test_case(
        1,
        1 + MAX_ENTRIES as u64,
        LookupTableStatus::Deactivated;
        "d1::cooldown_expired"
    )]
    // Here we hit branch `self.deactivation_slot == current_slot`.
    #[test_case(
        512,
        512,
        LookupTableStatus::Deactivating { remaining_blocks: MAX_ENTRIES }; // 512
        "d512::deactivated_in_current_slot"
    )]
    // Here `calculate_slot_position` returns `Some(0)`.
    #[test_case(
        512,
        512 + 1,
        LookupTableStatus::Deactivating { remaining_blocks: MAX_ENTRIES - 1 }; // 511
        "d512::deactivated_one_slot_ago"
    )]
    // Here `calculate_slot_position` returns `None`.
    #[test_case(
        512,
        512 + MAX_ENTRIES as u64,
        LookupTableStatus::Deactivated;
        "d512::cooldown_expired"
    )]
    fn test_lookup_table_meta_status(
        deactivation_slot: Slot,
        current_slot: Slot,
        expected_status: LookupTableStatus,
    ) {
        let meta = LookupTableMeta {
            deactivation_slot,
            ..LookupTableMeta::default()
        };
        assert_eq!(meta.status(current_slot), expected_status,);
    }

    #[test]
    fn test_overwrite_meta_data() {
        let meta = LookupTableMeta::new_for_tests();
        let empty_table = ProgramState::LookupTable(meta.clone());
        let mut serialized_table_1 = bincode::serialize(&empty_table).unwrap();
        serialized_table_1.resize(LOOKUP_TABLE_META_SIZE, 0);

        let address_table = AddressLookupTable::new_for_tests(meta, 0);
        let mut serialized_table_2 = vec![0; LOOKUP_TABLE_META_SIZE];
        AddressLookupTable::overwrite_meta_data(&mut serialized_table_2, address_table.meta)
            .unwrap();

        assert_eq!(serialized_table_1, serialized_table_2);
    }

    #[test]
    fn test_deserialize() {
        assert_eq!(
            AddressLookupTable::deserialize(&[]).err(),
            Some(ProgramError::InvalidAccountData),
        );

        assert_eq!(
            AddressLookupTable::deserialize(&[0u8; LOOKUP_TABLE_META_SIZE]).err(),
            Some(ProgramError::UninitializedAccount),
        );

        fn test_case(num_addresses: usize) {
            let lookup_table_meta = LookupTableMeta::new_for_tests();
            let address_table = AddressLookupTable::new_for_tests(lookup_table_meta, num_addresses);
            let address_table_data =
                AddressLookupTable::serialize_for_tests(address_table.clone()).unwrap();
            assert_eq!(
                AddressLookupTable::deserialize(&address_table_data).unwrap(),
                address_table,
            );
        }

        for case in [0, 1, 10, 255, 256] {
            test_case(case);
        }
    }

    #[test]
    fn test_lookup_from_empty_table() {
        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta::default(),
            addresses: Cow::Owned(vec![]),
        };

        assert_eq!(lookup_table.lookup(0, &[]), Ok(vec![]));
        assert_eq!(
            lookup_table.lookup(0, &[0]),
            Err(AddressLookupError::InvalidLookupIndex)
        );
    }

    #[test]
    fn test_serialize_new_lookup_table() {
        let authority_key = Pubkey::new_unique();
        let check_meta = LookupTableMeta::new(authority_key);

        // Success proper data size.
        let mut data = vec![0; LOOKUP_TABLE_META_SIZE];
        assert_eq!(
            ProgramState::serialize_new_lookup_table(&mut data, &authority_key),
            Ok(())
        );
        let deserialized = AddressLookupTable::deserialize(&data).unwrap();
        assert_eq!(deserialized.meta, check_meta);
        assert!(deserialized.addresses.is_empty());

        // Will overwrite existing data
        let mut data = vec![7; LOOKUP_TABLE_META_SIZE];
        assert_eq!(
            ProgramState::serialize_new_lookup_table(&mut data, &authority_key),
            Ok(())
        );
        let deserialized = AddressLookupTable::deserialize(&data).unwrap();
        assert_eq!(deserialized.meta, check_meta);
        assert!(deserialized.addresses.is_empty());

        // Fail data too small.
        let mut data = vec![0; 5];
        assert_eq!(
            ProgramState::serialize_new_lookup_table(&mut data, &authority_key),
            Err(ProgramError::AccountDataTooSmall)
        );
    }

    #[test]
    fn test_deserialize_addresses_from_index_mut() {
        let authority_key = Pubkey::new_unique();

        // Alloc space for no addresses.
        let mut data = vec![0; LOOKUP_TABLE_META_SIZE];
        ProgramState::serialize_new_lookup_table(&mut data, &authority_key).unwrap();

        // Cannot deserialize from the addresses offset if there are no
        // addresses.
        // Note the program will realloc first, before attempting this.
        assert_eq!(
            AddressLookupTable::deserialize_addresses_from_index_mut(&mut data, 0),
            Err(ProgramError::InvalidArgument)
        );

        // Alloc space for two addresses.
        let mut data = vec![0; LOOKUP_TABLE_META_SIZE + 64];
        ProgramState::serialize_new_lookup_table(&mut data, &authority_key).unwrap();

        // Try to deserialize from an index out of range.
        assert_eq!(
            AddressLookupTable::deserialize_addresses_from_index_mut(&mut data, 2),
            Err(ProgramError::InvalidArgument)
        );

        // Deserialize from the first index.
        let addresses =
            AddressLookupTable::deserialize_addresses_from_index_mut(&mut data, 0).unwrap();

        // Add two new unique addresses.
        let pubkey1 = Pubkey::new_unique();
        let pubkey2 = Pubkey::new_unique();
        addresses[0] = pubkey1;
        addresses[1] = pubkey2;
        assert_eq!(&addresses, &[pubkey1, pubkey2]);
    }

    #[test]
    fn test_lookup_from_deactivating_table() {
        let current_slot = 1;
        let addresses = vec![Pubkey::new_unique()];
        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                deactivation_slot: current_slot,
                last_extended_slot: current_slot - 1,
                ..LookupTableMeta::default()
            },
            addresses: Cow::Owned(addresses.clone()),
        };

        assert_eq!(
            lookup_table.meta.status(current_slot),
            LookupTableStatus::Deactivating {
                remaining_blocks: MAX_ENTRIES
            }
        );

        assert_eq!(
            lookup_table.lookup(current_slot, &[0]),
            Ok(vec![addresses[0]]),
        );
    }

    #[test]
    fn test_lookup_from_deactivated_table() {
        let current_slot = (MAX_ENTRIES + 1) as Slot;
        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                deactivation_slot: 0,
                last_extended_slot: 0,
                ..LookupTableMeta::default()
            },
            addresses: Cow::Owned(vec![]),
        };

        assert_eq!(
            lookup_table.meta.status(current_slot),
            LookupTableStatus::Deactivated
        );
        assert_eq!(
            lookup_table.lookup(current_slot, &[0]),
            Err(AddressLookupError::LookupTableAccountNotFound)
        );
    }

    #[test]
    fn test_lookup_from_table_extended_in_current_slot() {
        let current_slot = 0;
        let addresses: Vec<_> = (0..2).map(|_| Pubkey::new_unique()).collect();
        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                last_extended_slot: current_slot,
                last_extended_slot_start_index: 1,
                ..LookupTableMeta::default()
            },
            addresses: Cow::Owned(addresses.clone()),
        };

        assert_eq!(
            lookup_table.lookup(current_slot, &[0]),
            Ok(vec![addresses[0]])
        );
        assert_eq!(
            lookup_table.lookup(current_slot, &[1]),
            Err(AddressLookupError::InvalidLookupIndex),
        );
    }

    #[test]
    fn test_lookup_from_table_extended_in_previous_slot() {
        let current_slot = 1;
        let addresses: Vec<_> = (0..10).map(|_| Pubkey::new_unique()).collect();
        let lookup_table = AddressLookupTable {
            meta: LookupTableMeta {
                last_extended_slot: current_slot - 1,
                last_extended_slot_start_index: 1,
                ..LookupTableMeta::default()
            },
            addresses: Cow::Owned(addresses.clone()),
        };

        assert_eq!(
            lookup_table.lookup(current_slot, &[0, 3, 1, 5]),
            Ok(vec![addresses[0], addresses[3], addresses[1], addresses[5]])
        );
        assert_eq!(
            lookup_table.lookup(current_slot, &[10]),
            Err(AddressLookupError::InvalidLookupIndex),
        );
    }
}
