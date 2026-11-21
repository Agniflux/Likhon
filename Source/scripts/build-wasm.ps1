# Builds the engine for the website (crates/likhon-wasm) into Web/Laravel/public/engine/ (root CLAUDE.md 5a).
# Needs the wasm32-unknown-unknown target and the wasm-bindgen CLI matching the crate (0.2.129). If it is missing:
#   cargo install wasm-bindgen-cli --version 0.2.129 --root target/tools --bin wasm-bindgen
# (wasm-pack's own install fails on this PC with "cannot move the file to a different disk drive".)
$ErrorActionPreference = 'Stop'
$root = Resolve-Path (Join-Path $PSScriptRoot '..')
$crate = Join-Path $root 'crates\likhon-wasm'
$out = Join-Path $root '..\..\Web\Laravel\public\engine'
$bindgen = Join-Path $root 'target\tools\bin\wasm-bindgen.exe'
if (-not (Test-Path $bindgen)) { $bindgen = 'wasm-bindgen' }
cargo build --release --target wasm32-unknown-unknown --manifest-path (Join-Path $crate 'Cargo.toml')
if ($LASTEXITCODE) { exit $LASTEXITCODE }
New-Item -ItemType Directory -Force $out | Out-Null
& $bindgen (Join-Path $crate 'target\wasm32-unknown-unknown\release\likhon_wasm.wasm') --target web --no-typescript --out-dir $out --out-name likhon
if ($LASTEXITCODE) { exit $LASTEXITCODE }
Get-ChildItem $out | Select-Object Name, Length
