param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$workspace = Get-Content -LiteralPath (Join-Path $root 'Cargo.toml') -Raw -Encoding UTF8
$implementation = Get-Content -LiteralPath (Join-Path $root 'crates/sico-index/src/lib.rs') -Raw -Encoding UTF8

if ($workspace -notmatch 'crates/sico-index' -or $workspace -notmatch 'serde') {
  throw 'sico-index is not fully registered in the compiler workspace'
}
foreach ($contract in @('sico.semantic-index.v0', 'sico.semantic-query.v0', 'sico.semantic-response.v0', 'sha256:')) {
  if (-not $implementation.Contains($contract)) {
    throw "sico-index implementation is missing contract marker: $contract"
  }
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-index --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0028 index/query tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

& (Join-Path $root 'tools/validate-semantic-query.ps1') -RepositoryRoot $root
if ($LASTEXITCODE -ne 0) { throw 'RFC-0002 fixture regression failed' }

Write-Output 'STEP_0028_OK complete_modules=10 representative_partial=10 invalid_blocked=E2001 operations=5 snapshot=sha256 schema=index-query-response-v0 stable_ids=pass ranges=pass budgets=pass'
