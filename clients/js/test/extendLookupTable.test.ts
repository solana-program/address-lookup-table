import { generateKeyPairSigner } from '@solana/kit';
import { expect, it } from 'vitest';

import { createTestClient } from './_setup';

it('extends a lookup table with addresses and address wrappers', async () => {
    // Given an empty LUT and two addresses to add.
    const client = await createTestClient();
    const recentSlot = await client.rpc.getSlot({ commitment: 'finalized' }).send();
    await client.addressLookupTable.instructions
        .createLookupTable({ authority: client.payer, payer: client.payer, recentSlot })
        .sendTransaction();
    const [lut] = await client.addressLookupTable.pdas.addressLookupTable({ authority: client.payer, recentSlot });
    const [first, second] = await Promise.all([generateKeyPairSigner(), generateKeyPairSigner()]);

    // When we extend the LUT passing one raw address and one HasAddress wrapper.
    await client.addressLookupTable.instructions
        .extendLookupTable({
            address: lut,
            authority: client.payer,
            payer: client.payer,
            addresses: [first.address, { address: second.address }],
        })
        .sendTransaction();

    // Then we expect the LUT to store both unwrapped addresses.
    const lutAccount = await client.addressLookupTable.accounts.addressLookupTable.fetch(lut);
    expect(lutAccount.data.addresses).toEqual([first.address, second.address]);
});
