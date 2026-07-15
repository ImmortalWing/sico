param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$map = Get-Content -LiteralPath (Join-Path $root 'diagnostics/semantic-case-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$ownedCodes = @('E2001', 'E2002', 'E2010', 'E2011', 'E2020')
$owned = @($map.cases | Where-Object { $ownedCodes -contains $_.code })
$valid = @(
  Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b/numbers-units/valid') -File -Filter '*.sico'
  Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b/nominal-invariants/valid') -File -Filter '*.sico'
)

if ($owned.Count -ne 9) {
  throw "expected 9 owned diagnostic cases, found $($owned.Count)"
}
if ($valid.Count -ne 7) {
  throw "expected 7 owned valid cases, found $($valid.Count)"
}
if (@($owned.code | Sort-Object -Unique).Count -ne 5) {
  throw 'STEP-0023 diagnostic code coverage is incomplete'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = 'stable'
try {
  & $CargoPath test --offline --locked -p sico-semantics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0023 semantic tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0023_OK valid=7 invalid=9 exact_primary=9 codes=E2001,E2002,E2010,E2011,E2020 nominal_identity=pass fields=pass invariant=constant-v0 stable_facts=pass cascades=bounded'
