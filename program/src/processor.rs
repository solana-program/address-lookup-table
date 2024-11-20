//! Program processor

use {
    crate::{
        check_id,
        error::AddressLookupTableError,
        instruction::AddressLookupTableInstruction,
        state::{
            AddressLookupTable, ProgramState, LOOKUP_TABLE_MAX_ADDRESSES, LOOKUP_TABLE_META_SIZE,
        },
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        clock::{Clock, Slot},
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::{Pubkey, PUBKEY_BYTES},
        rent::Rent,
        slot_hashes::MAX_ENTRIES,
        system_instruction,
        sysvar::{slot_hashes::SlotHashesSysvar, Sysvar},
    },
};

/// Activation status of a lookup table
#[derive(Debug, PartialEq, Eq, Clone)]
enum LookupTableStatus {
    Activated,
    Deactivating { remaining_blocks: usize },
    Deactivated,
}

// Return the current status of the lookup table
fn get_lookup_table_status(
    deactivation_slot: Slot,
    current_slot: Slot,
) -> Result<LookupTableStatus, ProgramError> {
    if deactivation_slot == Slot::MAX {
        Ok(LookupTableStatus::Activated)
    } else if deactivation_slot == current_slot {
        Ok(LookupTableStatus::Deactivating {
            remaining_blocks: MAX_ENTRIES.saturating_add(1),
        })
    } else if let Some(slot_position) = SlotHashesSysvar::position(&deactivation_slot)? {
        // Deactivation requires a cool-down period to give in-flight transactions
        // enough time to land and to remove indeterminism caused by transactions
        // loading addresses in the same slot when a table is closed. The
        // cool-down period is equivalent to the amount of time it takes for
        // a slot to be removed from the slot hash list.
        //
        // By using the slot hash to enforce the cool-down, there is a side effect
        // of not allowing lookup tables to be recreated at the same derived address
        // because tables must be created at an address derived from a recent slot.
        Ok(LookupTableStatus::Deactivating {
            remaining_blocks: MAX_ENTRIES.saturating_sub(slot_position),
        })
    } else {
        Ok(LookupTableStatus::Deactivated)
    }
}

// Maximum input buffer length that can be deserialized.
// See `solana_sdk::packet::PACKET_DATA_SIZE`.
const MAX_INPUT_LEN: usize = 1232;
// Maximum vector length for new keys to be appended to a lookup table,
// provided to the `ExtendLookupTable` instruction.
// See comments below for `safe_deserialize_instruction`.
//
// Take the maximum input length and subtract 4 bytes for the discriminator,
// 8 bytes for the vector length, then divide that by the size of a `Pubkey`.
const MAX_NEW_KEYS_VECTOR_LEN: usize = (MAX_INPUT_LEN - 4 - 8) / 32;

// Stub of `AddressLookupTableInstruction` for partial deserialization.
// Keep in sync with the program's instructions in `instructions`.
#[allow(clippy::enum_variant_names)]
#[cfg_attr(test, derive(strum_macros::EnumIter))]
#[derive(serde::Serialize, serde::Deserialize, PartialEq)]
enum InstructionStub {
    CreateLookupTable,
    FreezeLookupTable,
    ExtendLookupTable { vector_len: u64 },
    DeactivateLookupTable,
    CloseLookupTable,
}

// [Core BPF]: The original Address Lookup Table builtin leverages the
// `solana_sdk::program_utils::limited_deserialize` method to cap the length of
// the input buffer at `MAX_INPUT_LEN` (1232). As a result, any input buffer
// larger than `MAX_INPUT_LEN` will abort deserialization and return
// `InstructionError::InvalidInstructionData`.
//
// Howevever, since `ExtendLookupTable` contains a vector of `Pubkey`, the
// `limited_deserialize` method will still read the vector's length and attempt
// to allocate a vector of the designated size. For extremely large length
// values, this can cause the initial allocation of a large vector to exhuast
// the BPF program's heap before deserialization can proceed.
//
// To mitigate this memory issue, the BPF version of the program has been
// designed to "peek" the length value for `ExtendLookupTable`, and ensure it
// cannot allocate a vector that would otherwise violate the input buffer
// length restriction.
fn safe_deserialize_instruction(
    input: &[u8],
) -> Result<AddressLookupTableInstruction, ProgramError> {
    match bincode::deserialize::<InstructionStub>(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?
    {
        InstructionStub::ExtendLookupTable { vector_len }
            if vector_len as usize > MAX_NEW_KEYS_VECTOR_LEN =>
        {
            return Err(ProgramError::InvalidInstructionData);
        }
        _ => {}
    }
    solana_program::program_utils::limited_deserialize(input, MAX_INPUT_LEN as u64)
        .map_err(|_| ProgramError::InvalidInstructionData)
}

// [Core BPF]: Feature "FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap"
// (relax_authority_signer_check_for_lookup_table_creation) is now enabled on
// all clusters, so the relevant checks have not been included in the Core BPF
// implementation.
// - Testnet:       Epoch 586
// - Devnet:        Epoch 591
// - Mainnet-Beta:  epoch 577
fn process_create_lookup_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    untrusted_recent_slot: Slot,
    bump_seed: u8,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;
    let authority_info = next_account_info(accounts_iter)?;
    let payer_info = next_account_info(accounts_iter)?;

    if !payer_info.is_signer {
        msg!("Payer account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let derivation_slot = {
        if SlotHashesSysvar::get(&untrusted_recent_slot)?.is_some() {
            Ok(untrusted_recent_slot)
        } else {
            msg!("{} is not a recent slot", untrusted_recent_slot);
            Err(ProgramError::InvalidInstructionData)
        }
    }?;

    // Use a derived address to ensure that an address table can never be
    // initialized more than once at the same address.
    let derived_table_seeds = &[
        authority_info.key.as_ref(),
        &derivation_slot.to_le_bytes(),
        &[bump_seed],
    ];
    let derived_table_key = Pubkey::create_program_address(derived_table_seeds, program_id)
        .map_err(AddressLookupTableError::from)?;

    if lookup_table_info.key != &derived_table_key {
        msg!(
            "Table address must match derived address: {}",
            derived_table_key
        );
        return Err(ProgramError::InvalidArgument);
    }

    // [Core BPF]: This check _is required_ since
    // "FKAcEvNgSY79RpqsPNUV5gDyumopH4cEHqUxyfm8b8Ap" was activated on
    // mainnet-beta.
    // See https://github.com/solana-labs/solana/blob/e4064023bf7936ced97b0d4de22137742324983d/programs/address-lookup-table/src/processor.rs#L129-L135.
    if check_id(lookup_table_info.owner) {
        return Ok(());
    }

    let lookup_table_data_len = LOOKUP_TABLE_META_SIZE;
    let rent = <Rent as Sysvar>::get()?;
    let required_lamports = rent
        .minimum_balance(lookup_table_data_len)
        .max(1)
        .saturating_sub(lookup_table_info.lamports());

    if required_lamports > 0 {
        invoke(
            &system_instruction::transfer(payer_info.key, lookup_table_info.key, required_lamports),
            &[payer_info.clone(), lookup_table_info.clone()],
        )?;
    }

    let _system_program_info = next_account_info(accounts_iter)?;

    invoke_signed(
        &system_instruction::allocate(lookup_table_info.key, lookup_table_data_len as u64),
        &[lookup_table_info.clone()],
        &[derived_table_seeds],
    )?;

    invoke_signed(
        &system_instruction::assign(lookup_table_info.key, program_id),
        &[lookup_table_info.clone()],
        &[derived_table_seeds],
    )?;

    ProgramState::serialize_new_lookup_table(
        &mut lookup_table_info.try_borrow_mut_data()?[..],
        authority_info.key,
    )?;

    Ok(())
}

fn process_freeze_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        msg!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let authority_info = next_account_info(accounts_iter)?;

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut lookup_table_meta = {
        let lookup_table_data = lookup_table_info.try_borrow_data()?;
        let lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is already frozen");
            return Err(ProgramError::Immutable);
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            msg!("Incorrect lookup table authority");
            return Err(ProgramError::IncorrectAuthority);
        }
        if lookup_table.meta.deactivation_slot != Slot::MAX {
            msg!("Deactivated tables cannot be frozen");
            return Err(ProgramError::InvalidArgument);
        }
        if lookup_table.addresses.is_empty() {
            msg!("Empty lookup tables cannot be frozen");
            return Err(ProgramError::InvalidInstructionData);
        }

        lookup_table.meta
    };

    lookup_table_meta.authority = None;
    AddressLookupTable::overwrite_meta_data(
        &mut lookup_table_info.try_borrow_mut_data()?[..],
        lookup_table_meta,
    )?;

    Ok(())
}

fn process_extend_lookup_table(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_addresses: Vec<Pubkey>,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        msg!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let authority_info = next_account_info(accounts_iter)?;

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (lookup_table_meta, new_addresses_start_index, new_table_data_len) = {
        let lookup_table_data = lookup_table_info.try_borrow_data()?;
        let mut lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is frozen");
            return Err(ProgramError::Immutable);
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            msg!("Incorrect lookup table authority");
            return Err(ProgramError::IncorrectAuthority);
        }
        if lookup_table.meta.deactivation_slot != Slot::MAX {
            msg!("Deactivated tables cannot be extended");
            return Err(ProgramError::InvalidArgument);
        }
        if lookup_table.addresses.len() >= LOOKUP_TABLE_MAX_ADDRESSES {
            msg!("Lookup table is full and cannot contain more addresses");
            return Err(ProgramError::InvalidArgument);
        }

        if new_addresses.is_empty() {
            msg!("Must extend with at least one address");
            return Err(ProgramError::InvalidInstructionData);
        }

        let new_table_addresses_len = lookup_table
            .addresses
            .len()
            .saturating_add(new_addresses.len());

        if new_table_addresses_len > LOOKUP_TABLE_MAX_ADDRESSES {
            msg!(
                "Extended lookup table length {} would exceed max capacity of {}",
                new_table_addresses_len,
                LOOKUP_TABLE_MAX_ADDRESSES,
            );
            return Err(ProgramError::InvalidInstructionData);
        }

        let old_table_addresses_len = u8::try_from(lookup_table.addresses.len()).map_err(|_| {
            // This is impossible as long as the length of new_addresses
            // is non-zero and LOOKUP_TABLE_MAX_ADDRESSES == u8::MAX + 1.
            ProgramError::InvalidAccountData
        })?;

        let clock = <Clock as Sysvar>::get()?;
        if clock.slot != lookup_table.meta.last_extended_slot {
            lookup_table.meta.last_extended_slot = clock.slot;
            lookup_table.meta.last_extended_slot_start_index = old_table_addresses_len;
        }

        let new_table_data_len = LOOKUP_TABLE_META_SIZE
            .checked_add(new_table_addresses_len.saturating_mul(PUBKEY_BYTES))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        (
            lookup_table.meta,
            old_table_addresses_len,
            new_table_data_len,
        )
    };

    // [Core BPF]:
    // When a builtin program attempts to write to an executable or read-only
    // account, it will be immediately rejected by the `TransactionContext`.
    // For more information, see https://github.com/solana-program/config/pull/21.
    //
    // However, in the case of the Address Lookup Table program's
    // `ExtendLookupTable` instruction, since the processor rejects any
    // zero-length "new keys" vectors, and will gladly append the same keys
    // again to the table, the issue here is slightly different than the linked
    // PR.
    //
    // The builtin version of the Address Lookup Table program will throw
    // when it attempts to overwrite the metadata, while the BPF version will
    // continue. In the case where an executable or read-only lookup table
    // account is provided, and some other requirement below is violated
    // (ie. no payer or system program accounts provided, payer is not a
    // signer, payer has insufficent balance, etc.), the BPF version will throw
    // based on one of those violations, rather than throwing immediately when
    // it encounters the executable or read-only lookup table account.
    //
    // In order to maximize backwards compatibility between the BPF version and
    // its original builtin, we add this check from `TransactionContext` to the
    // program directly, to throw even when the data being written is the same
    // same as what's currently in the account.
    //
    // Since the account can never be executable and also owned by the ALT
    // program, we'll just focus on readonly.
    if !lookup_table_info.is_writable {
        return Err(AddressLookupTableError::ReadonlyDataModified.into());
    }

    AddressLookupTable::overwrite_meta_data(
        &mut lookup_table_info.try_borrow_mut_data()?[..],
        lookup_table_meta,
    )?;

    lookup_table_info.realloc(new_table_data_len, false)?;

    {
        let mut lookup_table_data = lookup_table_info.try_borrow_mut_data()?;
        let uninitialized_addresses = AddressLookupTable::deserialize_addresses_from_index_mut(
            &mut lookup_table_data,
            new_addresses_start_index,
        )?;
        uninitialized_addresses.copy_from_slice(&new_addresses);
    }

    let rent = <Rent as Sysvar>::get()?;
    let required_lamports = rent
        .minimum_balance(new_table_data_len)
        .max(1)
        .saturating_sub(lookup_table_info.lamports());

    if required_lamports > 0 {
        let payer_info = next_account_info(accounts_iter)?;

        if !payer_info.is_signer {
            msg!("Payer account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        let _system_program_info = next_account_info(accounts_iter)?;

        invoke(
            &system_instruction::transfer(payer_info.key, lookup_table_info.key, required_lamports),
            &[payer_info.clone(), lookup_table_info.clone()],
        )?;
    }

    Ok(())
}

fn process_deactivate_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        msg!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let authority_info = next_account_info(accounts_iter)?;

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let mut lookup_table_meta = {
        let lookup_table_data = lookup_table_info.try_borrow_data()?;
        let lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is frozen");
            return Err(ProgramError::Immutable);
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            msg!("Incorrect lookup table authority");
            return Err(ProgramError::IncorrectAuthority);
        }
        if lookup_table.meta.deactivation_slot != Slot::MAX {
            msg!("Lookup table is already deactivated");
            return Err(ProgramError::InvalidArgument);
        }

        lookup_table.meta
    };

    let clock = <Clock as Sysvar>::get()?;

    lookup_table_meta.deactivation_slot = clock.slot;

    AddressLookupTable::overwrite_meta_data(
        &mut lookup_table_info.try_borrow_mut_data()?[..],
        lookup_table_meta,
    )?;

    Ok(())
}

fn process_close_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        msg!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    let authority_info = next_account_info(accounts_iter)?;

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let recipient_info = next_account_info(accounts_iter)?;

    // [Core BPF]: Here the legacy built-in version of ALT fallibly checks to
    // ensure the number of instruction accounts is 3.
    // It also checks that the recipient account is not the same as the lookup
    // table account.
    // The built-in does this by specifically checking the account keys at
    // their respective indices in the instruction context.
    // In BPF, we can just compare the addresses directly.
    if lookup_table_info.key == recipient_info.key {
        msg!("Lookup table cannot be the recipient of reclaimed lamports");
        return Err(ProgramError::InvalidArgument);
    }

    {
        let lookup_table_data = lookup_table_info.try_borrow_data()?;
        let lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is frozen");
            return Err(ProgramError::Immutable);
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            msg!("Incorrect lookup table authority");
            return Err(ProgramError::IncorrectAuthority);
        }

        let clock = <Clock as Sysvar>::get()?;

        match get_lookup_table_status(lookup_table.meta.deactivation_slot, clock.slot)? {
            LookupTableStatus::Activated => {
                msg!("Lookup table is not deactivated");
                Err(ProgramError::InvalidArgument)
            }
            LookupTableStatus::Deactivating { remaining_blocks } => {
                msg!(
                    "Table cannot be closed until it's fully deactivated in {} blocks",
                    remaining_blocks
                );
                Err(ProgramError::InvalidArgument)
            }
            LookupTableStatus::Deactivated => Ok(()),
        }?;
    }

    let new_recipient_lamports = lookup_table_info
        .lamports()
        .checked_add(recipient_info.lamports())
        .ok_or::<ProgramError>(ProgramError::ArithmeticOverflow)?;

    if !recipient_info.is_writable {
        return Err(AddressLookupTableError::ReadonlyLamportsChanged.into());
    }

    **recipient_info.try_borrow_mut_lamports()? = new_recipient_lamports;

    if !lookup_table_info.is_writable {
        return Err(AddressLookupTableError::ReadonlyDataModified.into());
    }

    // Lookup tables are _not_ reassigned when closed.
    lookup_table_info.realloc(0, true)?;
    **lookup_table_info.try_borrow_mut_lamports()? = 0;

    Ok(())
}

/// Processes a
/// `solana_programs_address_lookup_table::instruction::AddressLookupTableInstruction`
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = safe_deserialize_instruction(input)?;
    match instruction {
        AddressLookupTableInstruction::CreateLookupTable {
            recent_slot,
            bump_seed,
        } => {
            msg!("Instruction: CreateLookupTable");
            process_create_lookup_table(program_id, accounts, recent_slot, bump_seed)
        }
        AddressLookupTableInstruction::FreezeLookupTable => {
            msg!("Instruction: FreezeLookupTable");
            process_freeze_lookup_table(program_id, accounts)
        }
        AddressLookupTableInstruction::ExtendLookupTable { new_addresses } => {
            msg!("Instruction: ExtendLookupTable");
            process_extend_lookup_table(program_id, accounts, new_addresses)
        }
        AddressLookupTableInstruction::DeactivateLookupTable => {
            msg!("Instruction: DeactivateLookupTable");
            process_deactivate_lookup_table(program_id, accounts)
        }
        AddressLookupTableInstruction::CloseLookupTable => {
            msg!("Instruction: CloseLookupTable");
            process_close_lookup_table(program_id, accounts)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_instruction_serialization(
        stub: &InstructionStub,
        instruction: &AddressLookupTableInstruction,
        len: usize,
    ) {
        assert_eq!(
            bincode::serialize(&stub).unwrap(),
            bincode::serialize(&instruction).unwrap()[0..len],
        )
    }

    #[test]
    fn test_instruction_stubs() {
        assert_eq!(
            <InstructionStub as strum::IntoEnumIterator>::iter().count(),
            <AddressLookupTableInstruction as strum::IntoEnumIterator>::iter().count(),
        );

        assert_instruction_serialization(
            &InstructionStub::CreateLookupTable,
            &AddressLookupTableInstruction::CreateLookupTable {
                recent_slot: 0,
                bump_seed: 0,
            },
            4,
        );
        assert_instruction_serialization(
            &InstructionStub::FreezeLookupTable,
            &AddressLookupTableInstruction::FreezeLookupTable,
            4,
        );
        assert_instruction_serialization(
            &InstructionStub::ExtendLookupTable { vector_len: 4 },
            &AddressLookupTableInstruction::ExtendLookupTable {
                new_addresses: vec![Pubkey::new_unique(); 4],
            },
            12, // Check the vector length as well.
        );
        assert_instruction_serialization(
            &InstructionStub::DeactivateLookupTable,
            &AddressLookupTableInstruction::DeactivateLookupTable,
            4,
        );
        assert_instruction_serialization(
            &InstructionStub::CloseLookupTable,
            &AddressLookupTableInstruction::CloseLookupTable,
            4,
        );
    }
}
