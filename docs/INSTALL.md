# Installation

Every supported way to get the simulator running, from easiest to most manual.

> The simulator is a terminal application. You need a Rust toolchain (installed
> automatically by the npm and script paths) or Docker.

## 1. npm (the "npm way")

Requires [Node.js](https://nodejs.org) 16+ and a Rust toolchain
([rustup.rs](https://rustup.rs) — the npm `postinstall` pre-builds with cargo;
if Rust is missing it warns, and `npx` retries the build on first run).

```sh
# from the registry (once published)
npm install -g mini-os-kernel-simulator
mini-os-kernel

# or run without a global install
npx mini-os-kernel-simulator

# or straight from the repo, no registry needed
npm install -g github:Gunjan00001/Mini-OS-Kernel-Simulator
mini-os-kernel
```

Equivalent: `npm install -g Gunjan00001/Mini-OS-Kernel-Simulator`.

## 2. cargo (native Rust install)

```sh
cargo install --git https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator.git --locked mini-os-kernel
mini-os-kernel
```

## 3. One-command scripts

**macOS / Linux**

```sh
curl -fsSL https://raw.githubusercontent.com/Gunjan00001/Mini-OS-Kernel-Simulator/main/install.sh | bash
```

**Windows (PowerShell)**

```powershell
irm https://raw.githubusercontent.com/Gunjan00001/Mini-OS-Kernel-Simulator/main/install.ps1 | iex
```

Both install Rust via rustup if it is missing (the Windows script uses the
GNU toolchain so Visual Studio Build Tools are not required), then
`cargo install` from the repo.

## 4. Docker

```sh
docker build -t mini-os-kernel .
docker run -it --rm mini-os-kernel
```

`-it` matters: the simulator is an interactive menu.

## 5. Prebuilt binary (GitHub Releases)

Download `mini-os-kernel-*.zip` from
<https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator/releases>, unzip, and
run `mini-os-kernel` (`mini-os-kernel.exe` on Windows).

## 6. From source

```sh
git clone https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator.git
cd Mini-OS-Kernel-Simulator
cargo run --release
```

Tests: `cargo test` (76 tests).

## Package-manager registries (how to publish)

The repo ships the manifests needed by these registries; publishing requires
the maintainer's account.

| Registry | Command after packaging | Notes |
|---|---|---|
| **npm** | `npm publish` | `package.json` at the root is ready; run from a clean checkout. |
| **Homebrew** | `brew install --HEAD <tap>` | Add a formula pointing at the repo with `head "https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator.git"` and `depends_on "rust" => :build`. |
| **Scoop (Windows)** | `scoop install <bucket>/mini-os-kernel` | JSON manifest with `"checkver": "github"` and a `cargo`-based `installer`. |
| **Chocolatey** | `choco install mini-os-kernel` | `.nuspec` + `chocolateyinstall.ps1` that downloads the release zip. |
| **winget** | `winget install MiniOSKernel` | Submit a YAML manifest referencing the release installer. |

## Troubleshooting

- **`cargo: command not found`** — install Rust: <https://rustup.rs>, then
  restart the terminal.
- **Windows: `link.exe not found`** — install Visual Studio Build Tools, or use
  the GNU toolchain (`rustup default stable-x86_64-pc-windows-gnu`), which is
  what `install.ps1` does.
- **npm build fails behind a proxy** — set `CARGO_HTTP_PROXY`; the wrapper
  prints the cargo error verbatim.
- **Garbled box characters** — use a UTF-8 terminal; the simulator's output is
  pure ASCII, so any console works.
- **Piped input exits early** — by design: EOF terminates the simulator instead
  of spinning (fixed in v1.0.1).
