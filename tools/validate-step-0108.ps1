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
$work = Join-Path $root 'target\evidence\step-0108'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force

# --- REPL purity and AI tooling surface contract tests ---
Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked', '-p', 'sico-cli', 'step0108'
) 'repl-purity-test-failed|STEP-0108'
Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked', '-p', 'sico-ai-tools', 'step0108'
) 'ai-surface-contract-test-failed|STEP-0108'

# --- release build first (STEP-0098+ precedent for console fixtures) ---
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
if ($LASTEXITCODE -ne 0) { throw "runner-integration-tests-failed|STEP-0108 (see $runnerLog)" }
$log = Get-Content $runnerLog -Raw
foreach ($name in @(
        'successive_generations_teardown_cleanly_before_the_next_store',
        'hundred_runs_with_changing_grants_show_no_leak_or_authority_drift',
        'debug_pause_terminate_leaves_no_stranded_tasks_or_workers',
        'dap_hundred_sequential_sessions_have_bounded_rss_handles_and_no_poisoning'
    )) {
    if ($log -notmatch "test $name \.\.\.") { throw "missing test execution: $name" }
}
foreach ($marker in @('GRANT_MATRIX_100', 'DAP_TERMINATE_20')) {
    if ($log -notmatch [regex]::Escape($marker)) { throw "missing evidence marker: $marker" }
}

# --- regression gates: STEP-0107 chain (0106/0105/0104/0090/0087) + watch + REPL + DAP ---
foreach ($gate in @('validate-step-0107.ps1', 'validate-step-0091.ps1', 'validate-step-0100.ps1')) {
    & (Join-Path $root "tools\$gate") -RepositoryRoot $root
    if (-not $?) { throw "$gate regression under STEP-0108" }
}

Write-Output 'STEP_0108_OK debug-teardown=uniform watch-generations=isolated repl=pure-constant grants-matrix=100 dap-terminate=no-strand ai-surface=data-only gates=0107/0091/0100'
