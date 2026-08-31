param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$map = Get-Content -LiteralPath (Join-Path $root 'diagnostics/semantic-case-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$ownedCodes = @('E3001', 'E3002', 'E3003', 'E3101', 'E3102', 'E3103', 'E3104')
$owned = @($map.cases | Where-Object { $ownedCodes -contains $_.code })
$valid = @(
  Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b/exhaustive-match/valid') -File -Filter '*.sico'
  Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b/result-mapping/valid') -File -Filter '*.sico'
)

if ($owned.Count -ne 8) { throw "expected 8 owned diagnostic cases, found $($owned.Count)" }
if ($valid.Count -ne 6) { throw "expected 6 owned valid cases, found $($valid.Count)" }
if (@($owned.code | Sort-Object -Unique).Count -ne 7) {
  throw 'STEP-0024 diagnostic code coverage is incomplete'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-semantics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0024 semantic tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0024_OK valid=6 invalid=8 exact_primary=8 codes=E3001,E3002,E3003,E3101,E3102,E3103,E3104 sealed_coverage=pass result_flow=pass stable_facts=pass cascades=bounded'
