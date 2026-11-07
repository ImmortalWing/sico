param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$map = Get-Content -LiteralPath (Join-Path $root 'diagnostics/semantic-case-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$ownedCodes = @('E4001', 'E4002', 'E6001', 'E6002')
$owned = @($map.cases | Where-Object { $ownedCodes -contains $_.code })
$valid = @(
  Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b/effects-capabilities/valid') -File -Filter '*.sico'
  Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b/component-call/valid') -File -Filter '*.sico'
)

# Current-tree census: STEP-0025 (M2, 2026-07) recorded owned=4 / valid=4 /
# unique-codes=4. diagnostics/semantic-case-map.json grew 33 -> 46 with the
# 235-entry corpus refreeze + STEP-0584's 5-case oracle registration; the
# E4001 group gained IND-004, so owned is now 5 (E4001/E4002/E6001/E6002
# still cover exactly 4 distinct codes). Measured 2026-11-07: owned=5 /
# valid=4 / unique=4. The b/effects-capabilities and b/component-call valid
# dirs are unchanged (4 files).
if ($owned.Count -ne 5) { throw "expected 5 owned diagnostic cases, found $($owned.Count)" }
if ($valid.Count -ne 4) { throw "expected 4 owned valid cases, found $($valid.Count)" }
if (@($owned.code | Sort-Object -Unique).Count -ne 4) {
  throw 'STEP-0025 diagnostic code coverage is incomplete'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-semantics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0025 semantic tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

# invalid = owned diagnostic cases (5 after IND-004); exact_primary = distinct
# codes (4). These are current-tree counts; the STEP-0025 record was 4/4/4.
Write-Output 'STEP_0025_OK valid=4 invalid=5 exact_primary=4 codes=E4001,E4002,E6001,E6002 capability_boundary=pass explicit_purity=pass component_layers=separate stable_facts=pass'
