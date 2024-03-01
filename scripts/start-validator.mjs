#!/usr/bin/env zx
import { spawn } from "node:child_process";
import fs from "node:fs";
import "zx/globals";
import {
  getCargo,
  getExternalProgramAddresses,
  getExternalProgramOutputDir,
  getProgramFolders,
} from "./utils.mjs";

// Options and arguments.
const force = argv["force"];
const cliLogs = path.join(os.tmpdir(), "validator-cli.log");
const isValidatorRunning = (await $`lsof -t -i:8899`.quiet().exitCode) === 0;

// Keep the validator running when not using the force flag.
if (!force && isValidatorRunning) {
  echo(chalk.yellow("Local validator is already running."));
  process.exit();
}

// Initial message.
const verb = isValidatorRunning ? "Restarting" : "Starting";
const programs = [...getPrograms(), ...getExternalPrograms()];
const programPluralized = programs.length === 1 ? "program" : "programs";
echo(
  `${verb} local validator with ${programs.length} custom ${programPluralized}...`
);

// Kill the validator if it's already running.
if (isValidatorRunning) {
  await $`pkill -f solana-test-validator`.quiet();
  await sleep(1000);
}

// Global validator arguments.
const args = [/* Reset ledget */ "-r"];

// Load programs.
programs.forEach(({ programId, deployPath }) => {
  args.push(/* Load BPF program */ "--bpf-program", programId, deployPath);
});

// Start the validator in detached mode.
fs.writeFileSync(cliLogs, "", () => {});
const out = fs.openSync(cliLogs, "a");
const err = fs.openSync(cliLogs, "a");
const validator = spawn("solana-test-validator", args, {
  detached: true,
  stdio: ["ignore", out, err],
});
validator.unref();

// Wait for the validator to stabilize.
await spinner(
  "Waiting for local validator to stabilize...",
  () =>
    new Promise((resolve) => {
      setInterval(() => {
        const logs = fs.readFileSync(cliLogs, "utf8");
        if (logs.includes("Confirmed Slot: 1")) {
          fs.rmSync(cliLogs);
          resolve();
        }
      }, 1000);
    })
);

echo(chalk.green("Local validator is up and running!"));
process.exit();

function getPrograms() {
  const binaryDir = path.join(__dirname, "..", "target", "deploy");
  return getProgramFolders().map((folder) => {
    const cargo = getCargo(folder);
    const name = cargo.package.name.replace(/-/g, "_");
    return {
      programId: cargo.package.metadata.solana["program-id"],
      deployPath: path.join(binaryDir, `${name}.so`),
    };
  });
}

function getExternalPrograms() {
  const binaryDir = getExternalProgramOutputDir();
  return getExternalProgramAddresses().map((address) => ({
    programId: address,
    deployPath: path.join(binaryDir, `${address}.so`),
  }));
}
