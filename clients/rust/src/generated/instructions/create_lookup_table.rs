//! This code was AUTOGENERATED using the codama library.
//! Please DO NOT EDIT THIS FILE, instead use visitors
//! to add features, then rerun codama to update it.
//!
//! <https://github.com/codama-idl/codama>

use borsh::{BorshDeserialize, BorshSerialize};

/// Accounts.
#[derive(Debug)]
pub struct CreateLookupTable {
    pub address: solana_program::pubkey::Pubkey,

    pub authority: solana_program::pubkey::Pubkey,

    pub payer: solana_program::pubkey::Pubkey,

    pub system_program: solana_program::pubkey::Pubkey,
}

impl CreateLookupTable {
    pub fn instruction(
        &self,
        args: CreateLookupTableInstructionArgs,
    ) -> solana_program::instruction::Instruction {
        self.instruction_with_remaining_accounts(args, &[])
    }
    #[allow(clippy::vec_init_then_push)]
    pub fn instruction_with_remaining_accounts(
        &self,
        args: CreateLookupTableInstructionArgs,
        remaining_accounts: &[solana_program::instruction::AccountMeta],
    ) -> solana_program::instruction::Instruction {
        let mut accounts = Vec::with_capacity(4 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.address,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.authority,
            true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            self.payer, true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            self.system_program,
            false,
        ));
        accounts.extend_from_slice(remaining_accounts);
        let mut data = borsh::to_vec(&CreateLookupTableInstructionData::new()).unwrap();
        let mut args = borsh::to_vec(&args).unwrap();
        data.append(&mut args);

        solana_program::instruction::Instruction {
            program_id: crate::ADDRESS_LOOKUP_TABLE_ID,
            accounts,
            data,
        }
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateLookupTableInstructionData {
    discriminator: u32,
}

impl CreateLookupTableInstructionData {
    pub fn new() -> Self {
        Self { discriminator: 0 }
    }
}

impl Default for CreateLookupTableInstructionData {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(BorshSerialize, BorshDeserialize, Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CreateLookupTableInstructionArgs {
    pub recent_slot: u64,
    pub bump: u8,
}

/// Instruction builder for `CreateLookupTable`.
///
/// ### Accounts:
///
///   0. `[writable]` address
///   1. `[signer]` authority
///   2. `[writable, signer]` payer
///   3. `[optional]` system_program (default to
///      `11111111111111111111111111111111`)
#[derive(Clone, Debug, Default)]
pub struct CreateLookupTableBuilder {
    address: Option<solana_program::pubkey::Pubkey>,
    authority: Option<solana_program::pubkey::Pubkey>,
    payer: Option<solana_program::pubkey::Pubkey>,
    system_program: Option<solana_program::pubkey::Pubkey>,
    recent_slot: Option<u64>,
    bump: Option<u8>,
    __remaining_accounts: Vec<solana_program::instruction::AccountMeta>,
}

impl CreateLookupTableBuilder {
    pub fn new() -> Self {
        Self::default()
    }
    #[inline(always)]
    pub fn address(&mut self, address: solana_program::pubkey::Pubkey) -> &mut Self {
        self.address = Some(address);
        self
    }
    #[inline(always)]
    pub fn authority(&mut self, authority: solana_program::pubkey::Pubkey) -> &mut Self {
        self.authority = Some(authority);
        self
    }
    #[inline(always)]
    pub fn payer(&mut self, payer: solana_program::pubkey::Pubkey) -> &mut Self {
        self.payer = Some(payer);
        self
    }
    /// `[optional account, default to '11111111111111111111111111111111']`
    #[inline(always)]
    pub fn system_program(&mut self, system_program: solana_program::pubkey::Pubkey) -> &mut Self {
        self.system_program = Some(system_program);
        self
    }
    #[inline(always)]
    pub fn recent_slot(&mut self, recent_slot: u64) -> &mut Self {
        self.recent_slot = Some(recent_slot);
        self
    }
    #[inline(always)]
    pub fn bump(&mut self, bump: u8) -> &mut Self {
        self.bump = Some(bump);
        self
    }
    /// Add an additional account to the instruction.
    #[inline(always)]
    pub fn add_remaining_account(
        &mut self,
        account: solana_program::instruction::AccountMeta,
    ) -> &mut Self {
        self.__remaining_accounts.push(account);
        self
    }
    /// Add additional accounts to the instruction.
    #[inline(always)]
    pub fn add_remaining_accounts(
        &mut self,
        accounts: &[solana_program::instruction::AccountMeta],
    ) -> &mut Self {
        self.__remaining_accounts.extend_from_slice(accounts);
        self
    }
    #[allow(clippy::clone_on_copy)]
    pub fn instruction(&self) -> solana_program::instruction::Instruction {
        let accounts = CreateLookupTable {
            address: self.address.expect("address is not set"),
            authority: self.authority.expect("authority is not set"),
            payer: self.payer.expect("payer is not set"),
            system_program: self
                .system_program
                .unwrap_or(solana_program::pubkey!("11111111111111111111111111111111")),
        };
        let args = CreateLookupTableInstructionArgs {
            recent_slot: self.recent_slot.clone().expect("recent_slot is not set"),
            bump: self.bump.clone().expect("bump is not set"),
        };

        accounts.instruction_with_remaining_accounts(args, &self.__remaining_accounts)
    }
}

/// `create_lookup_table` CPI accounts.
pub struct CreateLookupTableCpiAccounts<'a, 'b> {
    pub address: &'b solana_program::account_info::AccountInfo<'a>,

    pub authority: &'b solana_program::account_info::AccountInfo<'a>,

    pub payer: &'b solana_program::account_info::AccountInfo<'a>,

    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
}

/// `create_lookup_table` CPI instruction.
pub struct CreateLookupTableCpi<'a, 'b> {
    /// The program to invoke.
    pub __program: &'b solana_program::account_info::AccountInfo<'a>,

    pub address: &'b solana_program::account_info::AccountInfo<'a>,

    pub authority: &'b solana_program::account_info::AccountInfo<'a>,

    pub payer: &'b solana_program::account_info::AccountInfo<'a>,

    pub system_program: &'b solana_program::account_info::AccountInfo<'a>,
    /// The arguments for the instruction.
    pub __args: CreateLookupTableInstructionArgs,
}

impl<'a, 'b> CreateLookupTableCpi<'a, 'b> {
    pub fn new(
        program: &'b solana_program::account_info::AccountInfo<'a>,
        accounts: CreateLookupTableCpiAccounts<'a, 'b>,
        args: CreateLookupTableInstructionArgs,
    ) -> Self {
        Self {
            __program: program,
            address: accounts.address,
            authority: accounts.authority,
            payer: accounts.payer,
            system_program: accounts.system_program,
            __args: args,
        }
    }
    #[inline(always)]
    pub fn invoke(&self) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed_with_remaining_accounts(&[], &[])
    }
    #[inline(always)]
    pub fn invoke_with_remaining_accounts(
        &self,
        remaining_accounts: &[(
            &'b solana_program::account_info::AccountInfo<'a>,
            bool,
            bool,
        )],
    ) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed_with_remaining_accounts(&[], remaining_accounts)
    }
    #[inline(always)]
    pub fn invoke_signed(
        &self,
        signers_seeds: &[&[&[u8]]],
    ) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed_with_remaining_accounts(signers_seeds, &[])
    }
    #[allow(clippy::clone_on_copy)]
    #[allow(clippy::vec_init_then_push)]
    pub fn invoke_signed_with_remaining_accounts(
        &self,
        signers_seeds: &[&[&[u8]]],
        remaining_accounts: &[(
            &'b solana_program::account_info::AccountInfo<'a>,
            bool,
            bool,
        )],
    ) -> solana_program::entrypoint::ProgramResult {
        let mut accounts = Vec::with_capacity(4 + remaining_accounts.len());
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.address.key,
            false,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.authority.key,
            true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new(
            *self.payer.key,
            true,
        ));
        accounts.push(solana_program::instruction::AccountMeta::new_readonly(
            *self.system_program.key,
            false,
        ));
        remaining_accounts.iter().for_each(|remaining_account| {
            accounts.push(solana_program::instruction::AccountMeta {
                pubkey: *remaining_account.0.key,
                is_signer: remaining_account.1,
                is_writable: remaining_account.2,
            })
        });
        let mut data = borsh::to_vec(&CreateLookupTableInstructionData::new()).unwrap();
        let mut args = borsh::to_vec(&self.__args).unwrap();
        data.append(&mut args);

        let instruction = solana_program::instruction::Instruction {
            program_id: crate::ADDRESS_LOOKUP_TABLE_ID,
            accounts,
            data,
        };
        let mut account_infos = Vec::with_capacity(5 + remaining_accounts.len());
        account_infos.push(self.__program.clone());
        account_infos.push(self.address.clone());
        account_infos.push(self.authority.clone());
        account_infos.push(self.payer.clone());
        account_infos.push(self.system_program.clone());
        remaining_accounts
            .iter()
            .for_each(|remaining_account| account_infos.push(remaining_account.0.clone()));

        if signers_seeds.is_empty() {
            solana_program::program::invoke(&instruction, &account_infos)
        } else {
            solana_program::program::invoke_signed(&instruction, &account_infos, signers_seeds)
        }
    }
}

/// Instruction builder for `CreateLookupTable` via CPI.
///
/// ### Accounts:
///
///   0. `[writable]` address
///   1. `[signer]` authority
///   2. `[writable, signer]` payer
///   3. `[]` system_program
#[derive(Clone, Debug)]
pub struct CreateLookupTableCpiBuilder<'a, 'b> {
    instruction: Box<CreateLookupTableCpiBuilderInstruction<'a, 'b>>,
}

impl<'a, 'b> CreateLookupTableCpiBuilder<'a, 'b> {
    pub fn new(program: &'b solana_program::account_info::AccountInfo<'a>) -> Self {
        let instruction = Box::new(CreateLookupTableCpiBuilderInstruction {
            __program: program,
            address: None,
            authority: None,
            payer: None,
            system_program: None,
            recent_slot: None,
            bump: None,
            __remaining_accounts: Vec::new(),
        });
        Self { instruction }
    }
    #[inline(always)]
    pub fn address(
        &mut self,
        address: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.address = Some(address);
        self
    }
    #[inline(always)]
    pub fn authority(
        &mut self,
        authority: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.authority = Some(authority);
        self
    }
    #[inline(always)]
    pub fn payer(&mut self, payer: &'b solana_program::account_info::AccountInfo<'a>) -> &mut Self {
        self.instruction.payer = Some(payer);
        self
    }
    #[inline(always)]
    pub fn system_program(
        &mut self,
        system_program: &'b solana_program::account_info::AccountInfo<'a>,
    ) -> &mut Self {
        self.instruction.system_program = Some(system_program);
        self
    }
    #[inline(always)]
    pub fn recent_slot(&mut self, recent_slot: u64) -> &mut Self {
        self.instruction.recent_slot = Some(recent_slot);
        self
    }
    #[inline(always)]
    pub fn bump(&mut self, bump: u8) -> &mut Self {
        self.instruction.bump = Some(bump);
        self
    }
    /// Add an additional account to the instruction.
    #[inline(always)]
    pub fn add_remaining_account(
        &mut self,
        account: &'b solana_program::account_info::AccountInfo<'a>,
        is_writable: bool,
        is_signer: bool,
    ) -> &mut Self {
        self.instruction
            .__remaining_accounts
            .push((account, is_writable, is_signer));
        self
    }
    /// Add additional accounts to the instruction.
    ///
    /// Each account is represented by a tuple of the `AccountInfo`, a `bool`
    /// indicating whether the account is writable or not, and a `bool`
    /// indicating whether the account is a signer or not.
    #[inline(always)]
    pub fn add_remaining_accounts(
        &mut self,
        accounts: &[(
            &'b solana_program::account_info::AccountInfo<'a>,
            bool,
            bool,
        )],
    ) -> &mut Self {
        self.instruction
            .__remaining_accounts
            .extend_from_slice(accounts);
        self
    }
    #[inline(always)]
    pub fn invoke(&self) -> solana_program::entrypoint::ProgramResult {
        self.invoke_signed(&[])
    }
    #[allow(clippy::clone_on_copy)]
    #[allow(clippy::vec_init_then_push)]
    pub fn invoke_signed(
        &self,
        signers_seeds: &[&[&[u8]]],
    ) -> solana_program::entrypoint::ProgramResult {
        let args = CreateLookupTableInstructionArgs {
            recent_slot: self
                .instruction
                .recent_slot
                .clone()
                .expect("recent_slot is not set"),
            bump: self.instruction.bump.clone().expect("bump is not set"),
        };
        let instruction = CreateLookupTableCpi {
            __program: self.instruction.__program,

            address: self.instruction.address.expect("address is not set"),

            authority: self.instruction.authority.expect("authority is not set"),

            payer: self.instruction.payer.expect("payer is not set"),

            system_program: self
                .instruction
                .system_program
                .expect("system_program is not set"),
            __args: args,
        };
        instruction.invoke_signed_with_remaining_accounts(
            signers_seeds,
            &self.instruction.__remaining_accounts,
        )
    }
}

#[derive(Clone, Debug)]
struct CreateLookupTableCpiBuilderInstruction<'a, 'b> {
    __program: &'b solana_program::account_info::AccountInfo<'a>,
    address: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    authority: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    payer: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    system_program: Option<&'b solana_program::account_info::AccountInfo<'a>>,
    recent_slot: Option<u64>,
    bump: Option<u8>,
    /// Additional instruction accounts `(AccountInfo, is_writable, is_signer)`.
    __remaining_accounts: Vec<(
        &'b solana_program::account_info::AccountInfo<'a>,
        bool,
        bool,
    )>,
}
