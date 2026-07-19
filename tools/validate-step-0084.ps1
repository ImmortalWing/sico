param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)

# 1. Workspace regression.
cargo fmt --all -- --check
if ($LASTEXITCODE -ne 0) { throw 'fmt failed' }
cargo clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
if ($LASTEXITCODE -ne 0) { throw 'clippy failed' }
cargo test --offline --locked --workspace --all-targets --all-features
if ($LASTEXITCODE -ne 0) { throw 'workspace tests failed' }

# 2. Benchmark matrix on the final pipeline (also rebuilds both binaries).
# Nearest-rank P95 needs at least 20 samples. With 10 samples P95 is the
# maximum, so one Windows first-touch outlier distorts the comparison.
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'tools\benchmark-step-0084.ps1') -Iterations 20
if ($LASTEXITCODE -ne 0) { throw 'benchmark failed' }
$report = Get-Content (Join-Path $root 'target\evidence\step-0084\benchmark.json') -Raw | ConvertFrom-Json
$goalComparison = if ($report.sico_run_cold_cache_miss_ms.p95 -lt 200 -and
    $report.sico_run_warm_cache_hit_ms.p95 -lt 120) { 'within-non-sla-goals' } else { 'measured-goal-miss' }

# 3. Audit artifacts exist.
foreach ($doc in @('docs\reports\m8-exit-audit-v0.md', 'docs\reports\script-standard-library-v0.md')) {
    if (-not (Test-Path (Join-Path $root $doc))) { throw "missing audit artifact $doc" }
}

Write-Output ("STEP_0084_OK cold_p95={0}ms warm_p95={1}ms runner_p95={2}ms comparison={3} gate=fmt,clippy,tests decision=GO" -f
    $report.sico_run_cold_cache_miss_ms.p95, $report.sico_run_warm_cache_hit_ms.p95,
    $report.runner_only_warm_component_ms.p95, $goalComparison)
