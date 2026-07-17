param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$workspace = Get-Content -LiteralPath (Join-Path $root 'Cargo.toml') -Raw -Encoding UTF8
$implementation = Get-Content -LiteralPath (Join-Path $root 'crates/sico-ir/src/lib.rs') -Raw -Encoding UTF8
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0008-typed-sico-ir-contract-v0.md') -Raw -Encoding UTF8

if (-not $workspace.Contains('crates/sico-ir')) { throw 'sico-ir is not registered in the compiler workspace' }
if ($rfc -notmatch '(?m)^> - status: accepted\r?$') { throw 'typed IR contract is not accepted' }
foreach ($contract in @('sico.ir.v0', 'require_semantic_success', 'pub fn verify', 'pub fn canonical_json', 'MAX_IR_DIAGNOSTICS')) {
  if (-not $implementation.Contains($contract)) { throw "sico-ir is missing contract marker: $contract" }
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-ir --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0030 IR/verifier tests failed' }
  & $CargoPath clippy --offline --locked -p sico-ir --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0030 strict Clippy failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0030_OK schema=sico.ir.v0 types=13 operations=16 terminators=4 valid_shapes=2 mutations=9 semantic_gate=E2001 frontend_gate=pass stable_ids=pass ranges=pass diagnostic_cap=100'
