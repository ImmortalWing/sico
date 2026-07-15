param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$properties = Get-Content -LiteralPath (Join-Path $root 'crates/sico-codegen-wasm/tests/backend.rs') -Raw -Encoding UTF8
$performance = Get-Content -LiteralPath (Join-Path $root 'tests/performance/m3-component-windows-release.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$audit = Get-Content -LiteralPath (Join-Path $root 'docs/reports/m3-exit-audit.md') -Raw -Encoding UTF8
$m4Plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M4-sapp-runtime.md') -Raw -Encoding UTF8
foreach ($bound in @('0..2_048_u32', '0..1_000_u32', 'compile_component(&ir).unwrap()', 'validate(&first)')) {
  if (-not $properties.Contains($bound)) { throw "M3 property/limit suite is missing: $bound" }
}
if ($performance.schema -ne 'sico.m3.component-performance-runs.v0' -or
    @($performance.runs).Count -ne 3 -or
    $performance.corpus.files -ne 3 -or
    $performance.iterations_per_run -ne 1000 -or
    $performance.builds_per_run -ne 3000 -or
    $performance.artifact_bytes_per_run -ne 303000 -or
    $performance.sla -ne 'not-established' -or
    $performance.median.elapsed_ms -ne 51.055 -or
    $performance.median.mib_per_second -ne 3.213) {
  throw 'M3 performance evidence is incomplete or overclaims an SLA'
}
if (-not $audit.Contains('GO: M3 complete; M4 entry gate satisfied; next STEP-0038.')) { throw 'M3 audit has no exact GO conclusion' }
if ($m4Plan -notmatch '(?m)^> - status: (?:ready after M3 GO|complete)\r?$' -or -not $m4Plan.Contains('STEP-0038')) { throw 'M4 plan has regressed below its M3-ready state' }

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath fmt --all -- --check
  if ($LASTEXITCODE -ne 0) { throw 'M3 formatting check failed' }
  & $CargoPath clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'M3 workspace strict Clippy failed' }
  & $CargoPath test --offline --locked --workspace --all-targets --all-features --quiet
  if ($LASTEXITCODE -ne 0) { throw 'M3 workspace tests failed' }
}
finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0037_OK scalar_inputs=2048 large_functions=1000 verifier_mutations=13 diagnostic_cap=100 runtime_corpus=3 perf_runs=3 perf_builds=3000 median_ms=51.055 median_mib_s=3.213 sla=not-established audit=GO next=STEP-0038'
