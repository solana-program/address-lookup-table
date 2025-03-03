/**
 * This code was AUTOGENERATED using the codama library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun codama to update it.
 *
 * @see https://github.com/codama-idl/codama
 */

import {
  combineCodec,
  getStructDecoder,
  getStructEncoder,
  getU32Decoder,
  getU32Encoder,
  transformEncoder,
  type Address,
  type Codec,
  type Decoder,
  type Encoder,
  type IAccountMeta,
  type IAccountSignerMeta,
  type IInstruction,
  type IInstructionWithAccounts,
  type IInstructionWithData,
  type ReadonlySignerAccount,
  type TransactionSigner,
  type WritableAccount,
} from '@solana/kit';
import { ADDRESS_LOOKUP_TABLE_PROGRAM_ADDRESS } from '../programs';
import { getAccountMetaFactory, type ResolvedAccount } from '../shared';

export const DEACTIVATE_LOOKUP_TABLE_DISCRIMINATOR = 3;

export function getDeactivateLookupTableDiscriminatorBytes() {
  return getU32Encoder().encode(DEACTIVATE_LOOKUP_TABLE_DISCRIMINATOR);
}

export type DeactivateLookupTableInstruction<
  TProgram extends string = typeof ADDRESS_LOOKUP_TABLE_PROGRAM_ADDRESS,
  TAccountAddress extends string | IAccountMeta<string> = string,
  TAccountAuthority extends string | IAccountMeta<string> = string,
  TRemainingAccounts extends readonly IAccountMeta<string>[] = [],
> = IInstruction<TProgram> &
  IInstructionWithData<Uint8Array> &
  IInstructionWithAccounts<
    [
      TAccountAddress extends string
        ? WritableAccount<TAccountAddress>
        : TAccountAddress,
      TAccountAuthority extends string
        ? ReadonlySignerAccount<TAccountAuthority> &
            IAccountSignerMeta<TAccountAuthority>
        : TAccountAuthority,
      ...TRemainingAccounts,
    ]
  >;

export type DeactivateLookupTableInstructionData = { discriminator: number };

export type DeactivateLookupTableInstructionDataArgs = {};

export function getDeactivateLookupTableInstructionDataEncoder(): Encoder<DeactivateLookupTableInstructionDataArgs> {
  return transformEncoder(
    getStructEncoder([['discriminator', getU32Encoder()]]),
    (value) => ({
      ...value,
      discriminator: DEACTIVATE_LOOKUP_TABLE_DISCRIMINATOR,
    })
  );
}

export function getDeactivateLookupTableInstructionDataDecoder(): Decoder<DeactivateLookupTableInstructionData> {
  return getStructDecoder([['discriminator', getU32Decoder()]]);
}

export function getDeactivateLookupTableInstructionDataCodec(): Codec<
  DeactivateLookupTableInstructionDataArgs,
  DeactivateLookupTableInstructionData
> {
  return combineCodec(
    getDeactivateLookupTableInstructionDataEncoder(),
    getDeactivateLookupTableInstructionDataDecoder()
  );
}

export type DeactivateLookupTableInput<
  TAccountAddress extends string = string,
  TAccountAuthority extends string = string,
> = {
  address: Address<TAccountAddress>;
  authority: TransactionSigner<TAccountAuthority>;
};

export function getDeactivateLookupTableInstruction<
  TAccountAddress extends string,
  TAccountAuthority extends string,
  TProgramAddress extends Address = typeof ADDRESS_LOOKUP_TABLE_PROGRAM_ADDRESS,
>(
  input: DeactivateLookupTableInput<TAccountAddress, TAccountAuthority>,
  config?: { programAddress?: TProgramAddress }
): DeactivateLookupTableInstruction<
  TProgramAddress,
  TAccountAddress,
  TAccountAuthority
> {
  // Program address.
  const programAddress =
    config?.programAddress ?? ADDRESS_LOOKUP_TABLE_PROGRAM_ADDRESS;

  // Original accounts.
  const originalAccounts = {
    address: { value: input.address ?? null, isWritable: true },
    authority: { value: input.authority ?? null, isWritable: false },
  };
  const accounts = originalAccounts as Record<
    keyof typeof originalAccounts,
    ResolvedAccount
  >;

  const getAccountMeta = getAccountMetaFactory(programAddress, 'programId');
  const instruction = {
    accounts: [
      getAccountMeta(accounts.address),
      getAccountMeta(accounts.authority),
    ],
    programAddress,
    data: getDeactivateLookupTableInstructionDataEncoder().encode({}),
  } as DeactivateLookupTableInstruction<
    TProgramAddress,
    TAccountAddress,
    TAccountAuthority
  >;

  return instruction;
}

export type ParsedDeactivateLookupTableInstruction<
  TProgram extends string = typeof ADDRESS_LOOKUP_TABLE_PROGRAM_ADDRESS,
  TAccountMetas extends readonly IAccountMeta[] = readonly IAccountMeta[],
> = {
  programAddress: Address<TProgram>;
  accounts: {
    address: TAccountMetas[0];
    authority: TAccountMetas[1];
  };
  data: DeactivateLookupTableInstructionData;
};

export function parseDeactivateLookupTableInstruction<
  TProgram extends string,
  TAccountMetas extends readonly IAccountMeta[],
>(
  instruction: IInstruction<TProgram> &
    IInstructionWithAccounts<TAccountMetas> &
    IInstructionWithData<Uint8Array>
): ParsedDeactivateLookupTableInstruction<TProgram, TAccountMetas> {
  if (instruction.accounts.length < 2) {
    // TODO: Coded error.
    throw new Error('Not enough accounts');
  }
  let accountIndex = 0;
  const getNextAccount = () => {
    const accountMeta = instruction.accounts![accountIndex]!;
    accountIndex += 1;
    return accountMeta;
  };
  return {
    programAddress: instruction.programAddress,
    accounts: {
      address: getNextAccount(),
      authority: getNextAccount(),
    },
    data: getDeactivateLookupTableInstructionDataDecoder().decode(
      instruction.data
    ),
  };
}
