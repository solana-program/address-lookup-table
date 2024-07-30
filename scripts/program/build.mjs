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
const buildArgs = ['--features', 'bpf-entrypoint', ...cliArguments()];

// Build the programs.
await Promise.all(
  getProgramFolders().map(async (folder) => {
    const manifestPath = path.join(workingDirectory, folder, 'Cargo.toml');

    await $`cargo-build-sbf --manifest-path ${manifestPath} ${buildArgs}`;
  })
);
