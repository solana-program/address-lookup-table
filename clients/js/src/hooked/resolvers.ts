import { Address } from '@solana/addresses';

export const resolveExtendLookupTableBytes = (scope: {
  args: { addresses: Array<Address> };
}): number => 32 * scope.args.addresses.length;
