# One-command installer for Windows (PowerShell).
# Installs Rust (via rustup, GNU toolchain -- no Visual Studio build tools
# required) if missing, then installs the simulator from git.
$ErrorActionPreference = "Stop"

$repo = if ($env:MINI_OS_REPO) { $env:MINI_OS_REPO } else { "https://github.com/Gunjan00001/Mini-OS-Kernel-Simulator.git" }

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Rust toolchain not found. Installing via rustup (GNU toolchain)..."
    $init = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-gnu/rustup-init.exe" -OutFile $init
    & $init -y --default-toolchain stable-x86_64-pc-windows-gnu
    $env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
}

Write-Host "Installing mini-os-kernel from $repo ..."
cargo install --git $repo --locked mini-os-kernel

Write-Host ""
Write-Host "Installed. Run:  mini-os-kernel"
