param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$performance = Get-Content -LiteralPath (Join-Path $root 'tests/performance/m1-frontend-windows-release.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($performance.schema -ne 'sico.m1.frontend-performance-runs.v0' -or @($performance.runs).Count -ne 3) {
  throw 'invalid M1 performance evidence envelope'
}
if ($performance.corpus.files -ne 70 -or $performance.corpus.canonical -ne 58 -or $performance.corpus.mutations -ne 12) {
  throw 'performance corpus does not match the M1 parser corpus'
}
if ($performance.iterations_per_run -ne 200 -or $performance.sla -ne 'not-established') {
  throw 'performance evidence must retain measured iterations and non-SLA scope'
}

$parser = Get-Content -LiteralPath (Join-Path $root 'crates/sico-parser/src/lib.rs') -Raw -Encoding UTF8
if (-not $parser.Contains('MAX_PARSE_DEPTH: usize = 256') -or -not $parser.Contains('MAX_PARSE_ERRORS: usize = 100')) {
  throw 'parser depth/error limits are not frozen at their reviewed values'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-parser -p sico-format --all-targets --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0021 property and limit tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output "STEP_0021_OK arbitrary_bytes=4096 valid_utf8=4096 depth_limit=256 parser_errors=100 canonical=58 mutations=12 perf_runs=3 perf_iterations=200 median_ms=$($performance.median.elapsed_ms) median_mib_s=$($performance.median.mib_per_second) sla=not-established"
