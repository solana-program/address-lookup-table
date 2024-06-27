//! Program processor

use {
    crate::{
        check_id,
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
        sysvar::Sysvar,
    },
};

/// Activation status of a lookup table
#[derive(Debug, PartialEq, Eq, Clone)]
enum LookupTableStatus {
    Activated,
    Deactivating { remaining_blocks: usize },
    Deactivated,
}

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
fn get_lookup_table_status(deactivation_slot: Slot, current_slot: Slot) -> LookupTableStatus {
    if deactivation_slot == Slot::MAX {
        LookupTableStatus::Activated
    } else if deactivation_slot == current_slot {
        LookupTableStatus::Deactivating {
            remaining_blocks: MAX_ENTRIES,
        }
    } else if let Some(slot_position) = calculate_slot_position(&deactivation_slot, &current_slot) {
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

// [Core BPF]: Locally-implemented
// `solana_sdk::program_utils::limited_deserialize`.
fn limited_deserialize<T>(input: &[u8]) -> Result<T, ProgramError>
where
    T: serde::de::DeserializeOwned,
{
    solana_program::program_utils::limited_deserialize(
        input, 1232, // [Core BPF]: See `solana_sdk::packet::PACKET_DATA_SIZE`
    )
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
    let _system_program_info = next_account_info(accounts_iter)?;

    if !payer_info.is_signer {
        msg!("Payer account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    // [Core BPF]: Since the `SlotHashes` sysvar is not available to BPF
    // programs, checking if a slot is a valid recent slot must be done
    // differently.
    // The `SlotHashes` sysvar stores up to `512` recent slots (`MAX_ENTRIES`).
    // We can instead use the `Clock` sysvar and do this math manually.
    //
    // Note this will no longer consider skipped slots wherein a block was not
    // produced.
    let derivation_slot = {
        let clock = <Clock as Sysvar>::get()?;
        let oldest_possible_slot = clock.slot.saturating_sub(MAX_ENTRIES as u64);
        if untrusted_recent_slot >= oldest_possible_slot && untrusted_recent_slot < clock.slot {
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
    let derived_table_key = Pubkey::create_program_address(derived_table_seeds, program_id)?;

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
    let authority_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        msg!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

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
    let authority_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        msg!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

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
        let _system_program_info = next_account_info(accounts_iter)?;

        if !payer_info.is_signer {
            msg!("Payer account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

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
    let authority_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        msg!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

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
    let authority_info = next_account_info(accounts_iter)?;
    let recipient_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
        msg!("Lookup table owner should be the Address Lookup Table program");
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

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

        // [Core BPF]: Again, since the `SlotHashes` sysvar is not available to
        // BPF programs, we can't use the `SlotHashes` sysvar to check the
        // status of a lookup table.
        // Again we instead use the `Clock` sysvar here.
        // This will no longer consider skipped slots wherein a block was not
        // produced.
        // See `state::LookupTableMeta::status` for more details.
        match get_lookup_table_status(lookup_table.meta.deactivation_slot, clock.slot) {
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

    **lookup_table_info.try_borrow_mut_lamports()? = 0;
    **recipient_info.try_borrow_mut_lamports()? = new_recipient_lamports;

    // Lookup tables are _not_ reassigned when closed.
    lookup_table_info.realloc(0, true)?;

    Ok(())
}

/// Processes a
/// `solana_programs_address_lookup_table::instruction::AddressLookupTableInstruction`
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = limited_deserialize(input)?;
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
