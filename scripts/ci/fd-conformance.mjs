#!/usr/bin/env zx

// Firedancer conformance testing of the Core BPF Config program against its
// original builtin implementation.
//
// Note: This script can only be run on Ubuntu.

import 'zx/globals';
import { getProgramId, getProgramSharedObjectPath, workingDirectory } from '../utils.mjs';

// Clone the conformance harness.
const harnessPath = path.join(workingDirectory, 'solana-conformance');
await $`git clone https://github.com/firedancer-io/solana-conformance.git`;

// Clone the test vectors.
const testVectorsPath = path.join(harnessPath, 'impl', 'test-vectors');
await $`git clone https://github.com/firedancer-io/test-vectors.git ${testVectorsPath}`;

// Add the Mollusk-generated fixtures to the test inputs.
const firedancerFixturesPath = path.join(testVectorsPath, 'instr', 'fixtures', 'address-lookup-table');
const molluskFixturesPath = path.join(workingDirectory, 'program', 'fuzz', 'blob');
await $`cp -a ${molluskFixturesPath}/. ${firedancerFixturesPath}/`;

// Remove the fixtures we want to skip.
const skipFixtures = [
    // In these fixtures, the builtin has > DEFAULT_COMPUTE_UNITS available, but
    // it goes over by CPI'ing to the System program to allocate & assign a new
    // lookup table.
    // It's extremely difficult and tedious to apply conformance special-casing to
    // conditional CPI-based CU consumption.
    // However, when the same CU constraints are applied directly on the BPF
    // program, it too will exhaust the meter and throw.
    '36e43433c2609890226ea8c5586b007071b96fa4_3246919.fix',
    '9ea61485a2f27361460ddfee9df3ba5dfde25dc5_3246919.fix',
    'a1a7bb8702d184ec9703498e2478c0a527e515f6_3246919.fix',
    // In this fixture, the ALT program itself is not provided as `executable`.
    // Until executable checks are relaxed, this is never going to work for BPF.
    // Builtins succeed because they need only the `native_loader::check_id()`
    // check.
    // The BPF loader's program invocation is where BPF programs' `executable`
    // flags are checked.
    // https://github.com/anza-xyz/agave/blob/970606e842d1027b5a34211219aacf3d39bb468d/programs/bpf_loader/src/lib.rs#L432-L436
    'b7e56ebaf6df34ab71ca9c1a42c0551cfb79dab7_2789718.fix',
    // This one belongs to the Stake program, but it's in the wrong folder.
    // See https://github.com/firedancer-io/test-vectors/pull/62.
    'crash-f2e925185043128e1cda0e21f2ab338321383ee4.fix',
];
for (const fixture of skipFixtures) {
    await $`rm -f ${path.join(firedancerFixturesPath, fixture)}`;
}

// Clone the SolFuzz-Agave harness.
const solFuzzAgavePath = path.join(harnessPath, 'impl', 'solfuzz-agave');
await $`git clone -b agave-v2.1.3 http://github.com/firedancer-io/solfuzz-agave.git ${solFuzzAgavePath}`;

// Fetch protobuf files.
await $`make -j -C ${solFuzzAgavePath} fetch_proto`

// Move into the conformance harness.
cd(harnessPath);

// Build the environment.
await $`bash install_ubuntu_lite.sh`;

const solFuzzAgaveManifestPath = path.join(solFuzzAgavePath, 'Cargo.toml');
const solFuzzAgaveTargetPath = path.join(
    solFuzzAgavePath,
    'target',
    'x86_64-unknown-linux-gnu',
    'release',
    'libsolfuzz_agave.so',
);

const testTargetsDir = path.join(harnessPath, 'impl', 'lib');
await $`mkdir -p ${testTargetsDir}`;

// Build the Agave target with the builtin version.
const testTargetPathBuiltin = path.join(testTargetsDir, 'builtin.so');
await $`cargo build --manifest-path ${solFuzzAgaveManifestPath} \
        --lib --release --target x86_64-unknown-linux-gnu`;
await $`mv ${solFuzzAgaveTargetPath} ${testTargetPathBuiltin}`;

// Build the Agave target with the BPF version.
const testTargetPathCoreBpf = path.join(testTargetsDir, 'core_bpf.so');
await $`CORE_BPF_PROGRAM_ID=${getProgramId('program')} \
        CORE_BPF_TARGET=${getProgramSharedObjectPath('program')} \
        FORCE_RECOMPILE=true \
        cargo build --manifest-path ${solFuzzAgaveManifestPath} \
        --lib --release --target x86_64-unknown-linux-gnu \
        --features core-bpf-conformance`;
await $`mv ${solFuzzAgaveTargetPath} ${testTargetPathCoreBpf}`;

// Remove any test results if they exist.
await $`rm -rf test_results`;

// Run the tests.
const fixturesPath = path.join(testVectorsPath, 'instr', 'fixtures', 'address-lookup-table');
await $`source test_suite_env/bin/activate && \
        solana-test-suite run-tests \
        -i ${fixturesPath} -s ${testTargetPathBuiltin} -t ${testTargetPathCoreBpf} \
        --core-bpf-mode --save-failures`;

// Assert conformance.
// There should be no fixtures in the `failed_protobufs` directory.
if (fs.existsSync('test_results/failed_protobufs')) {
    if (fs.readdirSync('test_results/failed_protobufs').length > 0) {
        throw new Error(`Error: mismatches detected.`);
    }
}

console.log('All tests passed.');
