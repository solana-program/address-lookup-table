#!/usr/bin/env zx
import 'zx/globals';
import {
  cliArguments,
  getToolchainArgument,
  popArgument,
  workingDirectory,
} from '../utils.mjs';

// Configure arguments here.
const lintArgs = [
  '-Zunstable-options',
  '--',
  '--deny=warnings',
  ...cliArguments(),
];

const fix = popArgument(lintArgs, '--fix');
const toolchain = getToolchainArgument('format');
const manifestPath = path.join(
  workingDirectory,
  'clients',
  'rust',
  'Cargo.toml'
);

// Check the client using Clippy.
if (fix) {
  await $`cargo ${toolchain} clippy --manifest-path ${manifestPath} --fix ${lintArgs}`;
} else {
  await $`cargo ${toolchain} clippy --manifest-path ${manifestPath} ${lintArgs}`;
}
