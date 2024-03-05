//! Program processor

use {
    crate::{
        check_id,
        instruction::ProgramInstruction,
        state::{
            AddressLookupTable, LookupTableStatus, ProgramState, LOOKUP_TABLE_MAX_ADDRESSES,
            LOOKUP_TABLE_META_SIZE,
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
        if untrusted_recent_slot > oldest_possible_slot && untrusted_recent_slot <= clock.slot {
            Ok(untrusted_recent_slot)
        } else {
            msg!("{} is not a recent slot", untrusted_recent_slot);
            Err(ProgramError::InvalidInstructionData)
        }
    }?;

    // Use a derived address to ensure that an address table can never be
    // initialized more than once at the same address.
    let derived_table_key = Pubkey::create_program_address(
        &[
            authority_info.key.as_ref(),
            &derivation_slot.to_le_bytes(),
            &[bump_seed],
        ],
        program_id,
    )?;

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
        &[&[
            authority_info.key.as_ref(),
            &derivation_slot.to_le_bytes(),
            &[bump_seed],
        ]],
    )?;

    invoke_signed(
        &system_instruction::assign(lookup_table_info.key, program_id),
        &[lookup_table_info.clone()],
        &[&[
            authority_info.key.as_ref(),
            &derivation_slot.to_le_bytes(),
            &[bump_seed],
        ]],
    )?;

    ProgramState::serialize_new_lookup_table(
        *lookup_table_info.try_borrow_mut_data()?,
        authority_info.key,
    )?;

    Ok(())
}

fn process_freeze_lookup_table(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();

    let lookup_table_info = next_account_info(accounts_iter)?;
    let authority_info = next_account_info(accounts_iter)?;

    if lookup_table_info.owner != program_id {
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
            // [Core BPF]: TODO: Should be `ProgramError::Immutable`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            // [Core BPF]: TODO: Should be `ProgramError::IncorrectAuthority`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
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
        *lookup_table_info.try_borrow_mut_data()?,
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
        return Err(ProgramError::InvalidAccountOwner);
    }

    if !authority_info.is_signer {
        msg!("Authority account must be a signer");
        return Err(ProgramError::MissingRequiredSignature);
    }

    let (lookup_table_meta, old_table_data_len, new_table_data_len) = {
        let lookup_table_data = lookup_table_info.try_borrow_data()?;
        let mut lookup_table = AddressLookupTable::deserialize(&lookup_table_data)?;

        if lookup_table.meta.authority.is_none() {
            msg!("Lookup table is frozen");
            // [Core BPF]: TODO: Should be `ProgramError::Immutable`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            // [Core BPF]: TODO: Should be `ProgramError::IncorrectAuthority`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
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

        let old_table_addresses_len = lookup_table.addresses.len();
        let new_table_addresses_len = old_table_addresses_len.saturating_add(new_addresses.len());

        if new_table_addresses_len > LOOKUP_TABLE_MAX_ADDRESSES {
            msg!(
                "Extended lookup table length {} would exceed max capacity of {}",
                new_table_addresses_len,
                LOOKUP_TABLE_MAX_ADDRESSES,
            );
            return Err(ProgramError::InvalidInstructionData);
        }

        let clock = <Clock as Sysvar>::get()?;
        if clock.slot != lookup_table.meta.last_extended_slot {
            lookup_table.meta.last_extended_slot = clock.slot;
            lookup_table.meta.last_extended_slot_start_index =
                u8::try_from(old_table_addresses_len).map_err(|_| {
                    // This is impossible as long as the length of new_addresses
                    // is non-zero and LOOKUP_TABLE_MAX_ADDRESSES == u8::MAX + 1.
                    ProgramError::InvalidAccountData
                })?;
        }

        let old_table_data_len = LOOKUP_TABLE_META_SIZE
            .checked_add(old_table_addresses_len.saturating_mul(PUBKEY_BYTES))
            .ok_or(ProgramError::ArithmeticOverflow)?;
        let new_table_data_len = LOOKUP_TABLE_META_SIZE
            .checked_add(new_table_addresses_len.saturating_mul(PUBKEY_BYTES))
            .ok_or(ProgramError::ArithmeticOverflow)?;

        (lookup_table.meta, old_table_data_len, new_table_data_len)
    };

    AddressLookupTable::overwrite_meta_data(
        *lookup_table_info.try_borrow_mut_data()?,
        lookup_table_meta,
    )?;

    lookup_table_info.realloc(new_table_data_len, false)?;

    {
        let mut lookup_table_data = lookup_table_info.try_borrow_mut_data()?;
        let uninitialized_addresses = AddressLookupTable::deserialize_addresses_from_index_mut(
            &mut lookup_table_data,
            old_table_data_len,
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
            // [Core BPF]: TODO: Should be `ProgramError::Immutable`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            // [Core BPF]: TODO: Should be `ProgramError::IncorrectAuthority`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
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
        *lookup_table_info.try_borrow_mut_data()?,
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
            // [Core BPF]: TODO: Should be `ProgramError::Immutable`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }
        if lookup_table.meta.authority != Some(*authority_info.key) {
            // [Core BPF]: TODO: Should be `ProgramError::IncorrectAuthority`
            // See https://github.com/solana-labs/solana/pull/35113
            return Err(ProgramError::Custom(0));
        }

        let clock = <Clock as Sysvar>::get()?;

        // [Core BPF]: Again, since the `SlotHashes` sysvar is not available to
        // BPF programs, we can't use the `SlotHashes` sysvar to check the
        // status of a lookup table.
        // Again we instead use the `Clock` sysvar here.
        // This will no longer consider skipped slots wherein a block was not
        // produced.
        // See `state::LookupTableMeta::status` for more details.
        match lookup_table.meta.status(clock.slot) {
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
/// `solana_programs_address_lookup_table::instruction::ProgramInstruction`
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = limited_deserialize(input)?;
    match instruction {
        ProgramInstruction::CreateLookupTable {
            recent_slot,
            bump_seed,
        } => {
            msg!("Instruction: CreateLookupTable");
            process_create_lookup_table(program_id, accounts, recent_slot, bump_seed)
        }
        ProgramInstruction::FreezeLookupTable => {
            msg!("Instruction: FreezeLookupTable");
            process_freeze_lookup_table(program_id, accounts)
        }
        ProgramInstruction::ExtendLookupTable { new_addresses } => {
            msg!("Instruction: ExtendLookupTable");
            process_extend_lookup_table(program_id, accounts, new_addresses)
        }
        ProgramInstruction::DeactivateLookupTable => {
            msg!("Instruction: DeactivateLookupTable");
            process_deactivate_lookup_table(program_id, accounts)
        }
        ProgramInstruction::CloseLookupTable => {
            msg!("Instruction: CloseLookupTable");
            process_close_lookup_table(program_id, accounts)
        }
    }
}
