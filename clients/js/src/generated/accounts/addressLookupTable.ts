/**
 * This code was AUTOGENERATED using the kinobi library.
 * Please DO NOT EDIT THIS FILE, instead use visitors
 * to add features, then rerun kinobi to update it.
 *
 * @see https://github.com/kinobi-so/kinobi
 */

import {
  assertAccountExists,
  assertAccountsExist,
  combineCodec,
  decodeAccount,
  fetchEncodedAccount,
  fetchEncodedAccounts,
  getAddressDecoder,
  getAddressEncoder,
  getArrayDecoder,
  getArrayEncoder,
  getOptionDecoder,
  getOptionEncoder,
  getStructDecoder,
  getStructEncoder,
  getU16Decoder,
  getU16Encoder,
  getU32Decoder,
  getU32Encoder,
  getU64Decoder,
  getU64Encoder,
  getU8Decoder,
  getU8Encoder,
  transformEncoder,
  type Account,
  type Address,
  type Codec,
  type Decoder,
  type EncodedAccount,
  type Encoder,
  type FetchAccountConfig,
  type FetchAccountsConfig,
  type MaybeAccount,
  type MaybeEncodedAccount,
  type Option,
  type OptionOrNullable,
} from '@solana/web3.js';
import { AddressLookupTableSeeds, findAddressLookupTablePda } from '../pdas';

export type AddressLookupTable = {
  discriminator: number;
  deactivationSlot: bigint;
  lastExtendedSlot: bigint;
  lastExtendedSlotStartIndex: number;
  authority: Option<Address>;
  padding: number;
  addresses: Array<Address>;
};

export type AddressLookupTableArgs = {
  deactivationSlot: number | bigint;
  lastExtendedSlot: number | bigint;
  lastExtendedSlotStartIndex: number;
  authority: OptionOrNullable<Address>;
  addresses: Array<Address>;
};

export function getAddressLookupTableEncoder(): Encoder<AddressLookupTableArgs> {
  return transformEncoder(
    getStructEncoder([
      ['discriminator', getU32Encoder()],
      ['deactivationSlot', getU64Encoder()],
      ['lastExtendedSlot', getU64Encoder()],
      ['lastExtendedSlotStartIndex', getU8Encoder()],
      [
        'authority',
        getOptionEncoder(getAddressEncoder(), { noneValue: 'zeroes' }),
      ],
      ['padding', getU16Encoder()],
      [
        'addresses',
        getArrayEncoder(getAddressEncoder(), { size: 'remainder' }),
      ],
    ]),
    (value) => ({ ...value, discriminator: 1, padding: 0 })
  );
}

export function getAddressLookupTableDecoder(): Decoder<AddressLookupTable> {
  return getStructDecoder([
    ['discriminator', getU32Decoder()],
    ['deactivationSlot', getU64Decoder()],
    ['lastExtendedSlot', getU64Decoder()],
    ['lastExtendedSlotStartIndex', getU8Decoder()],
    [
      'authority',
      getOptionDecoder(getAddressDecoder(), { noneValue: 'zeroes' }),
    ],
    ['padding', getU16Decoder()],
    ['addresses', getArrayDecoder(getAddressDecoder(), { size: 'remainder' })],
  ]);
}

export function getAddressLookupTableCodec(): Codec<
  AddressLookupTableArgs,
  AddressLookupTable
> {
  return combineCodec(
    getAddressLookupTableEncoder(),
    getAddressLookupTableDecoder()
  );
}

export function decodeAddressLookupTable<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress>
): Account<AddressLookupTable, TAddress>;
export function decodeAddressLookupTable<TAddress extends string = string>(
  encodedAccount: MaybeEncodedAccount<TAddress>
): MaybeAccount<AddressLookupTable, TAddress>;
export function decodeAddressLookupTable<TAddress extends string = string>(
  encodedAccount: EncodedAccount<TAddress> | MaybeEncodedAccount<TAddress>
):
  | Account<AddressLookupTable, TAddress>
  | MaybeAccount<AddressLookupTable, TAddress> {
  return decodeAccount(
    encodedAccount as MaybeEncodedAccount<TAddress>,
    getAddressLookupTableDecoder()
  );
}

export async function fetchAddressLookupTable<TAddress extends string = string>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<Account<AddressLookupTable, TAddress>> {
  const maybeAccount = await fetchMaybeAddressLookupTable(rpc, address, config);
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeAddressLookupTable<
  TAddress extends string = string,
>(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  address: Address<TAddress>,
  config?: FetchAccountConfig
): Promise<MaybeAccount<AddressLookupTable, TAddress>> {
  const maybeAccount = await fetchEncodedAccount(rpc, address, config);
  return decodeAddressLookupTable(maybeAccount);
}

export async function fetchAllAddressLookupTable(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<Account<AddressLookupTable>[]> {
  const maybeAccounts = await fetchAllMaybeAddressLookupTable(
    rpc,
    addresses,
    config
  );
  assertAccountsExist(maybeAccounts);
  return maybeAccounts;
}

export async function fetchAllMaybeAddressLookupTable(
  rpc: Parameters<typeof fetchEncodedAccounts>[0],
  addresses: Array<Address>,
  config?: FetchAccountsConfig
): Promise<MaybeAccount<AddressLookupTable>[]> {
  const maybeAccounts = await fetchEncodedAccounts(rpc, addresses, config);
  return maybeAccounts.map((maybeAccount) =>
    decodeAddressLookupTable(maybeAccount)
  );
}

export async function fetchAddressLookupTableFromSeeds(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  seeds: AddressLookupTableSeeds,
  config: FetchAccountConfig & { programAddress?: Address } = {}
): Promise<Account<AddressLookupTable>> {
  const maybeAccount = await fetchMaybeAddressLookupTableFromSeeds(
    rpc,
    seeds,
    config
  );
  assertAccountExists(maybeAccount);
  return maybeAccount;
}

export async function fetchMaybeAddressLookupTableFromSeeds(
  rpc: Parameters<typeof fetchEncodedAccount>[0],
  seeds: AddressLookupTableSeeds,
  config: FetchAccountConfig & { programAddress?: Address } = {}
): Promise<MaybeAccount<AddressLookupTable>> {
  const { programAddress, ...fetchConfig } = config;
  const [address] = await findAddressLookupTablePda(seeds, { programAddress });
  return await fetchMaybeAddressLookupTable(rpc, address, fetchConfig);
}
