#!/usr/bin/env zx

// Mollusk conformance testing of this Core BPF Address Lookup Table program
// against the version running on mainnet-beta.

import 'zx/globals';
import { getProgramId, getProgramSharedObjectPath, workingDirectory } from '../utils.mjs';

const programId = getProgramId('program');
const programBinaryPath = getProgramSharedObjectPath('program');
const baseBinaryDirPath = path.join(workingDirectory, 'target', 'dump-solana');
const baseBinaryPath = path.join(baseBinaryDirPath, 'base.so');
const molluskFixturesPath = path.join(workingDirectory, 'program', 'fuzz', 'blob');

// Clone the program from mainnet-beta.
// TODO: Switch to clone from mainnet-beta once feature is activated.
await $`mkdir -p ${baseBinaryDirPath}`;
await $`solana program dump -ud ${programId} ${baseBinaryPath}`;

// Test this program against the cloned program for conformance with Mollusk.
let output = await $`mollusk run-test \
    --proto firedancer \
    ${baseBinaryPath} ${programBinaryPath} \
    ${molluskFixturesPath} ${programId}`;

// The last line of output should exactly match the following:
// [DONE][TEST RESULT]: 0 failures
if (!output.stdout.includes("[DONE][TEST RESULT]: 0 failures")) {
    throw new Error(`Error: mismatches detected.`);
}
