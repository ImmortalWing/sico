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

if ($owned.Count -ne 4) { throw "expected 4 owned diagnostic cases, found $($owned.Count)" }
if ($valid.Count -ne 4) { throw "expected 4 owned valid cases, found $($valid.Count)" }
if (@($owned.code | Sort-Object -Unique).Count -ne 4) {
  throw 'STEP-0025 diagnostic code coverage is incomplete'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = 'stable'
try {
  & $CargoPath test --offline --locked -p sico-semantics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0025 semantic tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0025_OK valid=4 invalid=4 exact_primary=4 codes=E4001,E4002,E6001,E6002 capability_boundary=pass explicit_purity=pass component_layers=separate stable_facts=pass'
