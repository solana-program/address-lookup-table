import { Account, Address, some } from '@solana/kit';
import { expect, it } from 'vitest';

import { AddressLookupTable } from '../src';
import { createTestClient } from './_setup';

it('creates a new empty address lookup table', async () => {
    // Given a test client whose payer is funded with SOL and a recent slot.
    const client = await createTestClient();
    const recentSlot = await client.rpc.getSlot({ commitment: 'finalized' }).send();

    // When we create a new LUT using these parameters.
    await client.addressLookupTable.instructions
        .createLookupTable({ authority: client.payer.address, payer: client.payer, recentSlot })
        .sendTransaction();

    // Then a new account was created with the correct data.
    const [lut] = await client.addressLookupTable.pdas.addressLookupTable({
        authority: client.payer.address,
        recentSlot,
    });
    const lutAccount = await client.addressLookupTable.accounts.addressLookupTable.fetch(lut);
    expect(lutAccount).toMatchObject(<Account<AddressLookupTable>>{
        address: lut,
        data: {
            addresses: [] as Address[],
            authority: some(client.payer.address),
            deactivationSlot: BigInt(`0x${'ff'.repeat(8)}`),
            lastExtendedSlot: 0n,
            lastExtendedSlotStartIndex: 0,
        },
    });
});
