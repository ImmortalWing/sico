param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$properties = Get-Content -LiteralPath (Join-Path $root 'crates/sico-package/tests/security_properties.rs') -Raw -Encoding UTF8
$performance = Get-Content -LiteralPath (Join-Path $root 'tests/performance/m4-sapp-windows-release.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$audit = Get-Content -LiteralPath (Join-Path $root 'docs/reports/m4-exit-audit.md') -Raw -Encoding UTF8
$m5Plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M5-desktop-host.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0045-m4-security-quality-exit.md') -Raw -Encoding UTF8
foreach ($needle in @('0..2_048_usize', '0..1_024_usize', '0..256_usize', 'AllowUnsignedDevelopment', 'RequireDevelopment(BTreeSet::new())')) {
  if (-not $properties.Contains($needle)) { throw "M4 security property suite is missing: $needle" }
}
if ($performance.schema -ne 'sico.m4.sapp-performance-runs.v0' -or
    @($performance.runs).Count -ne 3 -or
    $performance.iterations_per_run -ne 1000 -or
    $performance.operations_per_run -ne 3000 -or
    $performance.sla -ne 'not-established' -or
    $performance.median.build_ms -ne 5.459 -or
    $performance.median.verify_ms -ne 8.571 -or
    $performance.median.signature_verify_ms -ne 48.177) {
  throw 'M4 performance evidence is incomplete or overclaims an SLA'
}
if (-not $audit.Contains('GO: M4 complete; M5 entry gate satisfied; next STEP-0046.')) { throw 'M4 audit has no exact GO' }
if ($m5Plan -notmatch '(?m)^> - status: (?:ready after M4 GO|complete)\r?$' -or -not $m5Plan.Contains('STEP-0053')) { throw 'M5 plan is not ready/complete' }
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0045 is not complete' }

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$previousRuntime = $env:SICO_TEST_WASMTIME
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  $env:SICO_TEST_WASMTIME = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
  & $CargoPath fmt --all -- --check
  if ($LASTEXITCODE -ne 0) { throw 'M4 formatting check failed' }
  & $CargoPath clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'M4 workspace strict Clippy failed' }
  & $CargoPath test --offline --locked --workspace --all-targets --all-features --quiet
  if ($LASTEXITCODE -ne 0) { throw 'M4 workspace tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
  $env:SICO_TEST_WASMTIME = $previousRuntime
}
Write-Output 'STEP_0045_OK signed_mutations=2048 component_mutations=1024 resource_permutations=256 threat_cases=18 perf_runs=3 perf_operations=3000 median_build_ms=5.459 median_verify_ms=8.571 median_signature_ms=48.177 sla=not-established audit=GO next=STEP-0046'
