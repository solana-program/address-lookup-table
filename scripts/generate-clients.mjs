#!/usr/bin/env zx
import 'zx/globals';
import { createFromRoot } from 'codama';
import { renderVisitor as renderJavaScriptVisitor } from '@codama/renderers-js';
import { renderVisitor as renderRustVisitor } from '@codama/renderers-rust';
import { parse as parseToml } from '@iarna/toml';

const workingDirectory = (await $`pwd`.quiet()).toString().trim();

function getCargo(folder) {
  return parseToml(
    fs.readFileSync(
      path.join(workingDirectory, folder ? folder : '.', 'Cargo.toml'),
      'utf8'
    )
  );
}

function getCargoMetadata(folder) {
  const cargo = getCargo(folder);
  return folder ? cargo?.package?.metadata : cargo?.workspace?.metadata;
}

function getToolchain(operation) {
  return getCargoMetadata()?.toolchains?.[operation];
}

function getToolchainArgument(operation) {
  const channel = getToolchain(operation);
  return channel ? `+${channel}` : '';
}

// Instanciate Codama.
const codama = createFromRoot(
  require(path.join(workingDirectory, 'program', 'idl.json'))
);

// Render JavaScript.
const jsClient = path.join(__dirname, '..', 'clients', 'js');
codama.accept(
  renderJavaScriptVisitor(path.join(jsClient, 'src', 'generated'), {
    prettier: require(path.join(jsClient, '.prettierrc.json')),
  })
);

// Render Rust.
const rustClient = path.join(__dirname, '..', 'clients', 'rust');
codama.accept(
  renderRustVisitor(path.join(rustClient, 'src', 'generated'), {
    anchorTraits: false,
    formatCode: true,
    crateFolder: rustClient,
    toolchain: getToolchainArgument('format'),
  })
);
