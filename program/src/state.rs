#[cfg(feature = "frozen-abi")]
use solana_frozen_abi_macro::{AbiEnumVisitor, AbiExample};
use {
    serde::{Deserialize, Serialize},
    solana_program::{clock::Slot, program_error::ProgramError, pubkey::Pubkey},
    std::borrow::Cow,
};

/// The maximum number of addresses that a lookup table can hold
pub const LOOKUP_TABLE_MAX_ADDRESSES: usize = 256;

/// The serialized size of lookup table metadata
pub const LOOKUP_TABLE_META_SIZE: usize = 56;

/// Address lookup table metadata
#[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct LookupTableMeta {
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
}

/// Program account states
#[cfg_attr(feature = "frozen-abi", derive(AbiExample, AbiEnumVisitor))]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
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

#[cfg_attr(feature = "frozen-abi", derive(AbiExample))]
#[derive(Debug, PartialEq, Eq, Clone)]
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
    use super::*;

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
}
