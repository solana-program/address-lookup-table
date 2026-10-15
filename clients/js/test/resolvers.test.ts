import { generateKeyPairSigner } from '@solana/kit';
import { expect, it } from 'vitest';

import { resolveExtendLookupTableBytes } from '../src/hooked';

it('resolves 32 bytes per address including address wrappers', async () => {
    // Given a raw address and an address wrapper.
    const [first, second] = await Promise.all([generateKeyPairSigner(), generateKeyPairSigner()]);

    // When we resolve the bytes needed to extend a LUT with both.
    const bytes = resolveExtendLookupTableBytes({ args: { addresses: [first.address, { address: second.address }] } });

    // Then we expect 32 bytes per address.
    expect(bytes).toBe(64);
});
