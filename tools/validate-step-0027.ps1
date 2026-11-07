param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$map = Get-Content -LiteralPath (Join-Path $root 'diagnostics/semantic-case-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$ownedCodes = @('E7001', 'E7002')
$owned = @($map.cases | Where-Object { $ownedCodes -contains $_.code })
$valid = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b/revision/valid') -File -Filter '*.sico')

# Current-tree census: STEP-0027 (M2, 2026-07) recorded owned=2 / valid=2 /
# unique-codes=2. diagnostics/semantic-case-map.json grew 33 -> 46 with the
# 235-entry corpus refreeze + STEP-0584's 5-case oracle registration; the
# E7002 group gained REV-201/203 and IND-005, so owned is now 5
# (E7001/E7002 still cover exactly 2 distinct codes), and b/revision/valid
# grew to 3 files. Measured 2026-11-07: owned=5 / valid=3 / unique=2.
if ($owned.Count -ne 5) { throw "expected 5 owned diagnostic cases, found $($owned.Count)" }
if ($valid.Count -ne 3) { throw "expected 3 owned valid cases, found $($valid.Count)" }
if (@($owned.code | Sort-Object -Unique).Count -ne 2) {
  throw 'STEP-0027 diagnostic code coverage is incomplete'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-semantics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0027 semantic tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

# invalid = owned diagnostic cases (5 after REV-201/203 + IND-005);
# exact_primary = distinct codes (2). These are current-tree counts; the
# STEP-0027 record was 2/2/2.
Write-Output 'STEP_0027_OK valid=3 invalid=5 exact_primary=2 codes=E7001,E7002 expected_revision=signature-derived stale_guard=dominating-branch stable_facts=pass runtime_conflict=absent'
