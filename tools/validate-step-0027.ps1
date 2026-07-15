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

if ($owned.Count -ne 2) { throw "expected 2 owned diagnostic cases, found $($owned.Count)" }
if ($valid.Count -ne 2) { throw "expected 2 owned valid cases, found $($valid.Count)" }
if (@($owned.code | Sort-Object -Unique).Count -ne 2) {
  throw 'STEP-0027 diagnostic code coverage is incomplete'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-semantics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0027 semantic tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0027_OK valid=2 invalid=2 exact_primary=2 codes=E7001,E7002 expected_revision=signature-derived stale_guard=dominating-branch stable_facts=pass runtime_conflict=absent'
