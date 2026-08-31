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
$work = Join-Path $root 'target\evidence\step-0105'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force

# --- scheduler unit tests (state machine, canonical order, ingress) ---
Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml'), '--lib', 'scheduler'
) 'scheduler-core-unit-tests-failed|STEP-0105'

# --- release build first: the runner integration suite follows the
# --- STEP-0098 precedent and runs in release, because the Windows console-
# --- control fixture races a fixed 150 ms CTRL_BREAK against cold debug-
# --- binary startup ---
Invoke-NativeChecked $cargo @(
    'build', '--release', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml')
) 'sico-runner build failed' | Out-Null

# --- runner integration suite, single-threaded so the Windows signal and
# --- warm-median fixtures are not perturbed by parallel load; scale and
# --- teardown metrics are captured as evidence ---
$runnerLog = Join-Path $work 'runner-tests.txt'
& $cargo test --release --offline --locked --manifest-path (Join-Path $runnerDir 'Cargo.toml') `
    --test runner -- --test-threads=1 --nocapture *> $runnerLog
# Native stderr progress output is redirected into the log, so $LASTEXITCODE
# is the pass/fail signal here ($? turns False on any stderr record in PS 5.1).
if ($LASTEXITCODE -ne 0) { throw "runner-integration-tests-failed|STEP-0105 (see $runnerLog)" }
$log = Get-Content $runnerLog -Raw
# Under --nocapture a printing test's output lands between `...` and `ok`,
# so presence plus absence of a FAILED marker is the honest per-test check;
# $LASTEXITCODE above already proves the suite result.
foreach ($name in @(
        'scheduler_scale_workloads_execute_within_run_bounds',
        'scheduler_limit_plus_one_cases_fail_with_typed_outcomes',
        'scheduler_adversarial_completion_records_fail_closed',
        'repeated_runs_show_no_task_handle_or_rss_growth_across_store_teardown',
        'guest_task_workloads_at_scale_run_within_bounds'
    )) {
    if ($log -notmatch "test $name \.\.\.") { throw "missing test execution: $name" }
}
foreach ($marker in @('SCHEDULER_SCALE tasks=1024', 'GUEST_TASK_SCALE tasks=1024', 'SCHEDULER_TEARDOWN_100')) {
    if ($log -notmatch [regex]::Escape($marker)) { throw "missing scale/teardown evidence: $marker" }
}

# --- typed fault surface is stable (Display strings are the contract) ---
$schedulerSource = Get-Content (Join-Path $runnerDir 'src\scheduler.rs') -Raw
foreach ($fault in @(
        'task table full (1024 live tasks)', 'task-group children exceeded (1024)',
        'scope nesting exceeded (64)', 'ready queue full (1024)',
        'host completion queue full (1024 records)', 'host completion bytes exceeded (16 MiB)',
        'scheduler metadata budget exceeded (16 MiB)', 'completion record belongs to another run or generation'
    )) {
    if ($schedulerSource -notlike "*$fault*") { throw "scheduler fault surface drifted: $fault" }
}

# --- end-to-end: the integrated runner still executes task programs exactly ---
Invoke-NativeChecked $cargo @('build', '-q', '--offline', '--locked', '-p', 'sico-cli') 'sico build failed'
$sico = Join-Path $root 'target\debug\sico.exe'
Invoke-NativeChecked $cargo @(
    'build', '--release', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml')
) 'sico-runner build failed' | Out-Null
$env:SICO_RUNNER = Join-Path $runnerDir 'target\release\sico-runner.exe'
$env:SICO_CACHE_DIR = Join-Path $work 'cache'
$collect = & $sico run (Join-Path $root 'tests\end-to-end\script-task-collect.sico') 2>$work\collect-stderr.txt
if ($LASTEXITCODE -ne 0 -or ($collect -join '') -cne 'alpha! beta!') { throw "task-collect failed: $($collect -join '')" }

# --- regression gates: sequential tasks, watch/persistence, semantic/IR contract ---
foreach ($gate in @('validate-step-0087.ps1', 'validate-step-0090.ps1', 'validate-step-0104.ps1')) {
    & (Join-Path $root "tools\$gate") -RepositoryRoot $root
    if (-not $?) { throw "$gate regression under STEP-0105" }
}

Write-Output 'STEP_0105_OK scheduler-core=green scale=1..1024 limits=typed adversarial=fail-closed teardown=stable e2e=exact step-0087/0090/0104=green'
