#!/usr/bin/env zx
import 'zx/globals';
import {
  cliArguments,
  getProgramFolders,
  workingDirectory,
} from '../utils.mjs';

// Save external programs binaries to the output directory.
import './dump.mjs';

// Configure arguments here.
const buildArgs = [
  '--features',
  'bpf-entrypoint',
  ...cliArguments()
];

// Build the programs.
for (const folder of getProgramFolders()) {
  const manifestPath = path.join(workingDirectory, folder, 'Cargo.toml');

  await $`cargo-build-sbf --manifest-path ${manifestPath} ${buildArgs}`;
}
