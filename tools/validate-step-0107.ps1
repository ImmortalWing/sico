param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
. (Join-Path $root 'tools\lib\native-command.ps1')

# --- toolchain hygiene: fmt, clippy ---
Invoke-NativeChecked $cargo @('fmt', '--all', '--check') 'cargo fmt check failed'
Invoke-NativeChecked $cargo @('clippy', '--workspace', '--all-targets', '--offline', '--locked', '--', '-D', 'warnings') 'clippy failed'

$runnerDir = Join-Path $root 'runner\sico-runner'
$work = Join-Path $root 'target\evidence\step-0107'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force

# --- scheduler unit tests: state machine, ingress, cancellation tree,
# --- select/race, deadlock, and the STEP-0107 channel fixtures ---
Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml'), '--lib', 'scheduler'
) 'scheduler-core-unit-tests-failed|STEP-0107'

# --- release build first (STEP-0098/0105/0106 precedent for the Windows
# --- console-control fixtures) ---
Invoke-NativeChecked $cargo @(
    'build', '--release', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml')
) 'sico-runner build failed' | Out-Null

# --- runner integration suite, single-threaded; metrics captured as evidence ---
$runnerLog = Join-Path $work 'runner-tests.txt'
& $cargo test --release --offline --locked --manifest-path (Join-Path $runnerDir 'Cargo.toml') `
    --test runner -- --test-threads=1 --nocapture *> $runnerLog
# Native stderr progress output is redirected into the log, so $LASTEXITCODE
# is the pass/fail signal here ($? turns False on any stderr record in PS 5.1).
if ($LASTEXITCODE -ne 0) { throw "runner-integration-tests-failed|STEP-0107 (see $runnerLog)" }
$log = Get-Content $runnerLog -Raw
# Under --nocapture a printing test's output lands between `...` and `ok`,
# so presence plus the suite-level exit code is the honest per-test check.
foreach ($name in @(
        'channel_relay_keeps_rss_independent_of_stream_size',
        'scheduler_cancellation_tree_scales_and_select_stays_canonical',
        'scheduler_scale_workloads_execute_within_run_bounds',
        'scheduler_adversarial_completion_records_fail_closed'
    )) {
    if ($log -notmatch "test $name \.\.\.") { throw "missing test execution: $name" }
}
if ($log -notmatch [regex]::Escape('CHANNEL_RELAY_1GIB')) { throw 'missing channel relay evidence' }

# --- typed fault surface additions are stable (Display is the contract) ---
$schedulerSource = Get-Content (Join-Path $runnerDir 'src\scheduler.rs') -Raw
foreach ($fault in @(
        'channel table full (1024 channels)', 'channel item capacity exceeded (1024)',
        'channel byte budget exceeded', 'unknown channel', 'is closed',
        'does not own channel', 'is already waiting on a channel'
    )) {
    if ($schedulerSource -notlike "*$fault*") { throw "scheduler fault surface drifted: $fault" }
}

# --- no new source surface: diagnostics/semantic-case oracles unchanged ---
& (Join-Path $root 'tools\validate-diagnostics.ps1') -RepositoryRoot $root
if (-not $?) { throw 'validate-diagnostics failed' }
& (Join-Path $root 'tools\validate-semantic-cases.ps1') -RepositoryRoot $root
if (-not $?) { throw 'validate-semantic-cases failed' }

# --- regression gate: STEP-0106 transitively reruns 0105/0104/0090/0087 ---
& (Join-Path $root 'tools\validate-step-0106.ps1') -RepositoryRoot $root
if (-not $?) { throw 'STEP-0106 regression under STEP-0107' }

Write-Output 'STEP_0107_OK channels=bounded rendezvous+fifo backpressure=suspend-not-spin close/move=typed cancel=failure-close relay-1GiB=rss-flat step-0106-chain=green'
