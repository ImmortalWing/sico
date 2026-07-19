param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot), [int]$Iterations = 20)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)

# Release binaries: the audit measures the shipping pipeline, not debug builds.
cargo build --release --offline --locked -p sico-cli
if ($LASTEXITCODE -ne 0) { throw 'sico-cli release build failed' }
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$runnerDir = Join-Path $root 'runner\sico-runner'
$build = "call `"$vcvars`" && set RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc&& cd /d `"$runnerDir`" && cargo build --release --offline"
$null = & cmd.exe /c $build
if ($LASTEXITCODE -ne 0) { throw 'sico-runner release build failed' }

$sico = Join-Path $root 'target\release\sico.exe'
$env:SICO_RUNNER = Join-Path $runnerDir 'target\release\sico-runner.exe'
$fixture = Join-Path $root 'tests\end-to-end\script-echo.sico'
$work = Join-Path $root 'target\evidence\step-0084'
New-Item -ItemType Directory -Force -Path $work | Out-Null
$runId = [Guid]::NewGuid().ToString('N')

function Measure-Run([string]$cacheDir) {
    $env:SICO_CACHE_DIR = $cacheDir
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $out = 'm8-audit-payload' | & cmd.exe /c "`"$sico`" run `"$fixture`" 2>NUL"
    $elapsed = $watch.Elapsed.TotalMilliseconds
    if ($LASTEXITCODE -ne 0 -or ($out -join '') -cne 'm8-audit-payload') { throw "benchmark run failed: $($out -join '')" }
    return $elapsed
}

function Stats([double[]]$values) {
    $sorted = $values | Sort-Object
    $n = $sorted.Count
    $p95 = $sorted[[Math]::Min([Math]::Ceiling($n * 0.95), $n) - 1]
    return [ordered]@{
        min = [Math]::Round($sorted[0], 4)
        median = [Math]::Round($sorted[[int]($n / 2)], 4)
        p95 = [Math]::Round($p95, 4)
        max = [Math]::Round($sorted[$n - 1], 4)
    }
}

# Cold: every iteration compiles into a fresh cache identity directory.
$cold = @()
for ($i = 0; $i -lt $Iterations; $i++) {
    $cold += Measure-Run (Join-Path $work "cold-cache-$runId-$i")
}
# Warm: one primed cache; every measured run is a pure cache hit.
$warmCache = Join-Path $work "warm-cache-$runId"
$null = Measure-Run $warmCache
$warm = @()
for ($i = 0; $i -lt $Iterations; $i++) {
    $warm += Measure-Run $warmCache
}
# Runner-only: invoke sico-runner directly on the cached component.
$entry = Get-ChildItem (Join-Path $warmCache 'script-v0') -File | Select-Object -First 1
if (-not $entry) { throw 'no cached component for runner-only measurement' }
$runnerOnly = @()
for ($i = 0; $i -lt ($Iterations * 2); $i++) {
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $out = 'm8-audit-payload' | & cmd.exe /c "`"$env:SICO_RUNNER`" `"$($entry.FullName)`" 2>NUL"
    $elapsed = $watch.Elapsed.TotalMilliseconds
    if ($LASTEXITCODE -ne 0 -or ($out -join '') -cne 'm8-audit-payload') { throw 'runner-only run failed' }
    $runnerOnly += $elapsed
}

$report = [ordered]@{
    benchmark = 'step-0084-m8-final-pipeline-v0'
    date = '2026-07-19'
    host = [System.Environment]::OSVersion.VersionString
    iterations = $Iterations
    method = 'wall-clock Stopwatch around sico run (script-echo) and direct sico-runner invocation; release binaries; fresh OS process per run'
    sico_run_cold_cache_miss_ms = Stats $cold
    sico_run_warm_cache_hit_ms = Stats $warm
    runner_only_warm_component_ms = Stats $runnerOnly
    note = 'cold includes full frontend+codegen+cache commit+runner; warm is a verified cache hit; runner-only isolates the execution boundary (compile excluded)'
}
$path = Join-Path $work 'benchmark.json'
$report | ConvertTo-Json -Depth 4 | Set-Content $path -Encoding utf8
Write-Output ("STEP_0084_BENCH_OK cold_p95={0}ms warm_p95={1}ms runner_p95={2}ms report={3}" -f
    $report.sico_run_cold_cache_miss_ms.p95, $report.sico_run_warm_cache_hit_ms.p95,
    $report.runner_only_warm_component_ms.p95, $path)
