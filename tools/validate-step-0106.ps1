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
$work = Join-Path $root 'target\evidence\step-0106'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force

# --- scheduler unit tests: state machine, ingress, cancellation tree,
# --- select/race matrices, deadlock proofs, 1,024-chain scale ---
Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml'), '--lib', 'scheduler'
) 'scheduler-core-unit-tests-failed|STEP-0106'

# --- release build first: the runner integration suite follows the
# --- STEP-0098/0105 precedent and runs in release, because the Windows
# --- console-control fixtures race a fixed 150 ms CTRL_BREAK against cold
# --- debug-binary startup ---
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
if ($LASTEXITCODE -ne 0) { throw "runner-integration-tests-failed|STEP-0106 (see $runnerLog)" }
$log = Get-Content $runnerLog -Raw
# Under --nocapture a printing test's output lands between `...` and `ok`,
# so presence plus the suite-level exit code is the honest per-test check.
foreach ($name in @(
        'scheduler_cancellation_tree_scales_and_select_stays_canonical',
        'cancelled_guest_run_drives_the_scheduler_tree_to_teardown',
        'scheduler_scale_workloads_execute_within_run_bounds',
        'scheduler_limit_plus_one_cases_fail_with_typed_outcomes',
        'scheduler_adversarial_completion_records_fail_closed'
    )) {
    if ($log -notmatch "test $name \.\.\.") { throw "missing test execution: $name" }
}
if ($log -notmatch [regex]::Escape('CHAIN_CANCEL_1024')) { throw 'missing 1,024-chain cancellation evidence' }

# --- typed fault surface additions are stable (Display is the contract) ---
$schedulerSource = Get-Content (Join-Path $runnerDir 'src\scheduler.rs') -Raw
foreach ($fault in @(
        'select/race operands exceeded (256)', 'select/race requires at least one operand',
        'select/race has no ready operand this turn',
        'suspended tasks with no runnable work or pending host operations'
    )) {
    if ($schedulerSource -notlike "*$fault*") { throw "scheduler fault surface drifted: $fault" }
}

# --- race/select source spellings stay refused (RFC-0036 §9): the
# --- diagnostics/semantic-case oracles must stay green with no new codes ---
& (Join-Path $root 'tools\validate-diagnostics.ps1') -RepositoryRoot $root
if (-not $?) { throw 'validate-diagnostics failed' }
& (Join-Path $root 'tools\validate-semantic-cases.ps1') -RepositoryRoot $root
if (-not $?) { throw 'validate-semantic-cases failed' }

# --- regression gates ---
foreach ($gate in @('validate-step-0087.ps1', 'validate-step-0090.ps1', 'validate-step-0104.ps1', 'validate-step-0105.ps1')) {
    & (Join-Path $root "tools\$gate") -RepositoryRoot $root
    if (-not $?) { throw "$gate regression under STEP-0106" }
}

Write-Output 'STEP_0106_OK cancel-tree=reverse-order select=canonical-256 matrices=one-result deadlock=typed chain-1024=bounded step-0087/0090/0104/0105=green'
