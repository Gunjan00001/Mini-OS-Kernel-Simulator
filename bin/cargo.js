"use strict";
// Shared helper: locate the cargo executable even when ~/.cargo/bin is not on
// PATH yet (the usual situation right after installing Rust via rustup).

const path = require("node:path");
const { spawnSync } = require("node:child_process");

function cargoBin() {
  const exe = process.platform === "win32" ? "cargo.exe" : "cargo";
  const home = process.env.USERPROFILE || process.env.HOME || "";
  const candidates = [
    process.env.CARGO,
    "cargo",
    home ? path.join(home, ".cargo", "bin", exe) : null,
  ].filter(Boolean);
  for (const candidate of candidates) {
    const probe = spawnSync(candidate, ["--version"], { stdio: "ignore" });
    if (probe.status === 0) return candidate;
  }
  return null;
}

module.exports = { cargoBin };
