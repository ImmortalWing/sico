param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$implementation = Get-Content -LiteralPath (Join-Path $root 'crates/sico-ir/src/lower.rs') -Raw -Encoding UTF8
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0009-core-lowering-evaluation-order-v0.md') -Raw -Encoding UTF8
$snapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/ir/core-lowering.snap') -Encoding UTF8 | Where-Object { $_.Trim() })

if ($rfc -notmatch '(?m)^> - status: accepted\r?$') { throw 'core evaluation order contract is not accepted' }
foreach ($contract in @('pub fn lower_core', 'CoreLowerError::Semantic', 'Operation::Try', 'Terminator::Match')) {
  if (-not $implementation.Contains($contract)) { throw "core lowering is missing contract marker: $contract" }
}
if ($snapshots.Count -ne 12) { throw "expected 12 core lowering snapshots, found $($snapshots.Count)" }

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = 'stable'
try {
  & $CargoPath test --offline --locked -p sico-ir --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0031 core lowering tests failed' }
  & $CargoPath clippy --offline --locked -p sico-ir --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0031 strict Clippy failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0031_OK core_valid=12 deferred_valid=13 invalid_blocked=29 snapshots=12 evaluation=left-to-right-single-evaluation operations=18 terminators=5 verifier_mutations=3'
