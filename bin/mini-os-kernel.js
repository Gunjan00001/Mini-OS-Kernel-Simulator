#!/usr/bin/env node
// Runs the simulator binary, building it with cargo on first use if needed.
"use strict";

const { spawnSync } = require("node:child_process");
const path = require("node:path");
const fs = require("node:fs");

const root = path.resolve(__dirname, "..");
const exe = path.join(
  root,
  "target",
  "release",
  process.platform === "win32" ? "mini-os-kernel.exe" : "mini-os-kernel"
);

function cargoBuild() {
  console.error("[mini-os-kernel] building (first run)...");
  const build = spawnSync("cargo", ["build", "--release", "--locked"], {
    cwd: root,
    stdio: "inherit",
  });
  if (build.error || build.status !== 0) {
    console.error(
      "[mini-os-kernel] build failed. Install Rust from https://rustup.rs and retry."
    );
    process.exit(1);
  }
}

if (!fs.existsSync(exe)) {
  cargoBuild();
}

const run = spawnSync(exe, process.argv.slice(2), { stdio: "inherit" });
process.exit(run.status === null ? 1 : run.status);
