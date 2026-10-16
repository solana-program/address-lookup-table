import type { Address, HasAddress } from '@solana/kit';

export const resolveExtendLookupTableBytes = (scope: { args: { addresses: Array<Address | HasAddress> } }): number =>
    32 * scope.args.addresses.length;
