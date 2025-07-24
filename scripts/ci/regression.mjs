#!/usr/bin/env zx

// Mollusk conformance testing of this Core BPF Address Lookup Table program
// against the version running on mainnet-beta.

import 'zx/globals';
import { getProgramId, getProgramSharedObjectPath, workingDirectory } from '../utils.mjs';

const programId = getProgramId('program');
const programBinaryPath = getProgramSharedObjectPath('program');
const baseBinaryPath = path.join(workingDirectory, 'program', 'fuzz', 'program-mb-3-17-2025.so');
const molluskFixturesPath = path.join(workingDirectory, 'program', 'fuzz', 'blob');

// Test this program against the cloned program for conformance with Mollusk.
let output = await $`mollusk run-test \
    --proto mollusk \
    --ignore-compute-units \
    ${baseBinaryPath} ${programBinaryPath} \
    ${molluskFixturesPath} ${programId}`;

// The last line of output should exactly match the following:
// [DONE][TEST RESULT]: 0 failures
if (!output.stdout.includes("[DONE][TEST RESULT]: 0 failures")) {
    throw new Error(`Error: mismatches detected.`);
}
