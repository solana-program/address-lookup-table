import type { Address } from '@solana/web3.js';

export const resolveExtendLookupTableBytes = (scope: {
  args: { addresses: Array<Address> };
}): number => 32 * scope.args.addresses.length;
