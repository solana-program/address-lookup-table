import {
  Address,
  appendTransactionMessageInstruction,
  pipe,
  Account,
  some,
} from '@solana/web3.js';
import test from 'ava';
import {
  AddressLookupTable,
  fetchAddressLookupTable,
  findAddressLookupTablePda,
  getCreateLookupTableInstructionAsync,
} from '../src';
import {
  createDefaultSolanaClient,
  createDefaultTransaction,
  generateKeyPairSignerWithSol,
  signAndSendTransaction,
} from './_setup';

test('it creates a new empty address lookup table', async (t) => {
  // Given an authority wallet with SOL and a recent slot.
  const client = createDefaultSolanaClient();
  const [authority, recentSlot] = await Promise.all([
    generateKeyPairSignerWithSol(client),
    client.rpc.getSlot({ commitment: 'finalized' }).send(),
  ]);

  // When we create a new LUT using these parameters.
  const createLut = await getCreateLookupTableInstructionAsync({
    authority,
    recentSlot,
  });
  await pipe(
    await createDefaultTransaction(client, authority),
    (tx) => appendTransactionMessageInstruction(createLut, tx),
    (tx) => signAndSendTransaction(client, tx)
  );

  // Then a new account was created with the correct data.
  const [lut] = await findAddressLookupTablePda({
    authority: authority.address,
    recentSlot,
  });
  const lutAccount = await fetchAddressLookupTable(client.rpc, lut);
  t.like(lutAccount, <Account<AddressLookupTable>>{
    address: lut,
    data: {
      addresses: [] as Address[],
      authority: some(authority.address),
      deactivationSlot: BigInt(`0x${'ff'.repeat(8)}`),
      lastExtendedSlot: 0n,
      lastExtendedSlotStartIndex: 0,
    },
  });
});
