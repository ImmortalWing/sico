param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$map = Get-Content -LiteralPath (Join-Path $root 'diagnostics/semantic-case-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$ownedCodes = @('E5001', 'E5002', 'E5101', 'E5102', 'E5201', 'E5202')
$owned = @($map.cases | Where-Object { $ownedCodes -contains $_.code })
$valid = @()
foreach ($group in @('affine-resources', 'future-task', 'stream')) {
  $valid += @(Get-ChildItem -LiteralPath (Join-Path $root "syntax-candidates/b/$group/valid") -File -Filter '*.sico')
}

if ($owned.Count -ne 6) { throw "expected 6 owned diagnostic cases, found $($owned.Count)" }
if ($valid.Count -ne 6) { throw "expected 6 owned valid cases, found $($valid.Count)" }
if (@($owned.code | Sort-Object -Unique).Count -ne 6) {
  throw 'STEP-0026 diagnostic code coverage is incomplete'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-semantics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0026 semantic tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0026_OK valid=6 invalid=6 exact_primary=6 codes=E5001,E5002,E5101,E5102,E5201,E5202 affine_flow=pass structured_tasks=pass stream_bounds=pass stable_facts=pass'
