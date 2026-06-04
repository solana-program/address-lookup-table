import path from 'node:path';

import { createClient, lamports } from '@solana/kit';
import { litesvm } from '@solana/kit-plugin-litesvm';
import { airdropSigner, generatedSigner } from '@solana/kit-plugin-signer';

import { ADDRESS_LOOKUP_TABLE_PROGRAM_ADDRESS, addressLookupTableProgram } from '../src';

const ADDRESS_LOOKUP_TABLE_BINARY_PATH = path.resolve(
    __dirname,
    '..',
    '..',
    '..',
    'target',
    'deploy',
    'solana_address_lookup_table_program.so',
);

export const createTestClient = () => {
    return createClient()
        .use(generatedSigner())
        .use(litesvm())
        .use(airdropSigner(lamports(1_000_000_000n)))
        .use(client => {
            // Load the address-lookup-table program into the LiteSVM instance
            // from its compiled `.so` file. This must run after the `litesvm()`
            // plugin so that `client.svm` is available.
            client.svm.addProgramFromFile(ADDRESS_LOOKUP_TABLE_PROGRAM_ADDRESS, ADDRESS_LOOKUP_TABLE_BINARY_PATH);
            return client;
        })
        .use(addressLookupTableProgram());
};
