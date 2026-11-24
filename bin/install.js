#!/usr/bin/env node
// npm postinstall: pre-build the Rust binary so `npx mini-os-kernel` is fast.
// A missing toolchain is only a warning -- `npx` retries the build on first run.
"use strict";

const { spawnSync } = require("node:child_process");
const path = require("node:path");

const root = path.resolve(__dirname, "..");

const check = spawnSync("cargo", ["--version"], { stdio: "ignore" });
if (check.status !== 0) {
  console.warn(
    "[mini-os-kernel] cargo not found: skipping pre-build.\n" +
      "  Install Rust from https://rustup.rs, then run: npx mini-os-kernel"
  );
  process.exit(0);
}

console.log("[mini-os-kernel] pre-building release binary...");
const build = spawnSync("cargo", ["build", "--release", "--locked"], {
  cwd: root,
  stdio: "inherit",
});
if (build.status !== 0) {
  console.warn(
    "[mini-os-kernel] pre-build failed; `npx mini-os-kernel` will retry the build."
  );
}
