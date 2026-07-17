param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cli = Get-Content -LiteralPath (Join-Path $root 'crates/sico-cli/src/lib.rs') -Raw -Encoding UTF8
$properties = Get-Content -LiteralPath (Join-Path $root 'crates/sico-semantics/tests/semantic_properties.rs') -Raw -Encoding UTF8
$performance = Get-Content -LiteralPath (Join-Path $root 'tests/performance/m2-semantics-windows-release.json') -Raw -Encoding UTF8 | ConvertFrom-Json

if ($cli.Contains('type checker: unavailable') -or $cli.Contains('"type_checker": "unavailable"')) {
  throw 'semantic CLI still advertises the M1 unavailable boundary'
}
foreach ($contract in @('semantic_checks_performed', 'sico.diagnostics.v0', 'sico_semantics::{', 'analyze(&source)')) {
  if (-not $cli.Contains($contract)) { throw "semantic CLI is missing contract marker: $contract" }
}
foreach ($bound in @('2_048_u32', 'MAX_SEMANTIC_DIAGNOSTICS', '0..2_000', '0..200')) {
  if (-not $properties.Contains($bound)) { throw "semantic property suite is missing bound: $bound" }
}
if ($performance.schema -ne 'sico.m2.semantics-performance-runs.v0' -or
    @($performance.runs).Count -ne 3 -or
    $performance.corpus.files -ne 54 -or
    $performance.corpus.valid -ne 25 -or
    $performance.corpus.invalid -ne 29 -or
    $performance.iterations_per_run -ne 200 -or
    $performance.analyses_per_run -ne 10800 -or
    $performance.sla -ne 'not-established' -or
    $performance.median.elapsed_ms -ne 1242.186 -or
    $performance.median.mib_per_second -ne 2.808) {
  throw 'M2 performance evidence is incomplete or overclaims an SLA'
}
foreach ($run in @($performance.runs)) {
  if ($run.accepted -ne 5000 -or $run.rejected -ne 5800) {
    throw "performance run $($run.run) does not cover the 25/29 oracle"
  }
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-cli -p sico-semantics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0029 CLI/property tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0029_OK cli_valid=25 cli_invalid=29 exact_primary=29 text_json=pass semantic_inputs=2048 diagnostic_cap=100 locals=2000 expression_depth=200 perf_runs=3 perf_iterations=200 median_ms=1242.186 median_mib_s=2.808 sla=not-established'
