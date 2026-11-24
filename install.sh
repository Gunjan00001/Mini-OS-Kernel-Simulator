#!/usr/bin/env bash
# One-command installer for macOS / Linux.
# Installs Rust (via rustup) if missing, then installs the simulator from git.
set -euo pipefail

REPO="${MINI_OS_REPO:-https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator.git}"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Rust toolchain not found. Installing via rustup..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
fi

echo "Installing mini-os-kernel from $REPO ..."
cargo install --git "$REPO" --locked mini-os-kernel

echo
echo "Installed. Run:  mini-os-kernel"
