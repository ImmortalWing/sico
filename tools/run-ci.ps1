# M19 §3.1: one-command CI — every claim in this repository re-runnable
# from a fresh clone. Usage:  powershell -File tools/run-ci.ps1 [-Fast]
#   -Fast skips clippy (the slowest step) for iteration; CI itself runs full.
param([switch]$Fast)
$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $false
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { throw 'cargo not found' }
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$env:SICO_TEST_WASMTIME = & (Join-Path $repo 'tools\ensure-wasmtime.ps1')

# Fresh-host prerequisites (STEP-0196). The GNU toolchain's dlltool lives
# in the repo-local MSYS2 binutils directory (the same candidate
# tools/validate-step-0034.ps1 resolves); the console-control fixtures
# need a stdlib python.exe; and packages_resolve.rs falls back to the
# runner debug exe, so the runner workspace must be built before the
# workspace test step on a fresh clone.
$binutils = Join-Path $repo 'target\tooling\msys2-binutils\mingw64\bin'
if (Test-Path -LiteralPath $binutils -PathType Container) {
    $env:Path = "$binutils;$env:Path"
}
$env:SICO_PYTHON = & (Join-Path $repo 'tools\ensure-python.ps1')
$pythonDir = Split-Path -Parent $env:SICO_PYTHON
$env:Path = "$pythonDir;$env:Path"

$failures = New-Object System.Collections.Generic.List[string]
$results = New-Object System.Collections.Generic.List[string]
function Invoke-Step([string]$name, [scriptblock]$body) {
    Write-Host "=== CI: $name" -ForegroundColor Cyan
    $timer = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $body 2>&1 | Out-String
    $code = $LASTEXITCODE
    $timer.Stop()
    $elapsed = $timer.Elapsed.ToString('hh\:mm\:ss')
    $results.Add(("{0}`t{1}`t{2}" -f $name, $elapsed, $code)) | Out-Null
    if ($code -ne 0) {
        $tail = $output.Substring([Math]::Max(0, $output.Length - 2000))
        $tail | Write-Host
        $failures.Add($name) | Out-Null
        Write-Host "=== FAIL: $name ($elapsed)" -ForegroundColor Red
        # The independent CI log needs a signed-in session to read, so the
        # failing step would otherwise be invisible to anyone without one.
        # GitHub renders workflow commands as annotations on the run page
        # without a session, so the failing step and the tail of its output
        # are published there. A line that starts with `::` in the step
        # output would be read as a workflow command of its own, so those
        # lines are neutralized first.
        $safe = ($tail -split "`r?`n" | ForEach-Object { if ($_ -match '^\s*::') { ':' + $_.TrimStart(':') } else { $_ } })
        Write-Host "::error title=CI step failed::$name"
        # A workflow command carries one line, and GitHub keeps a bounded
        # number of annotations per step, so the last lines of the step's
        # output are published one by one.
        $lines = @($safe | Where-Object { $_.Trim().Length -gt 0 } | Select-Object -Last 8)
        foreach ($line in $lines) {
            $short = $line
            if ($short.Length -gt 900) { $short = $short.Substring(0, 900) + ' ...' }
            Write-Host "::error title=CI step output::$short"
        }
    } else {
        Write-Host "=== PASS: $name ($elapsed)" -ForegroundColor Green
    }
}

Invoke-Step 'fmt' { & $cargo fmt --all -- --check; $LASTEXITCODE }
# The runner is a separate Cargo workspace and is not a member of the root
# workspace, so the root "cargo fmt --all" above does not cover it. Without
# this step a rustfmt violation under runner/ would pass the one-command CI
# and the independent runner alike (STEP-0499; only
# tools/validate-step-0245.ps1 checked it, and 0245 is not a CI step).
Invoke-Step 'fmt (runner)' { & $cargo fmt --manifest-path .\runner\sico-runner\Cargo.toml --all -- --check; $LASTEXITCODE }
if (-not $Fast) {
    Invoke-Step 'clippy (workspace)' { & $cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings; $LASTEXITCODE }
    Invoke-Step 'clippy (runner)' { & $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings; $LASTEXITCODE }
}
Invoke-Step 'build (runner debug)' { & $cargo build --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml; $LASTEXITCODE }
# The runner suite's RSS/handle budget asserts measure the *process-wide*
# metrics of the test binary; under the default threaded harness, sibling
# test allocations land inside those measurements (deterministic 83 MB
# false growth on this host). The suite is seconds long, so CI runs it
# serially; the budget constants themselves are unchanged.
#
# STEP-0536: the `runner` binary also carries the second env-sensitive budget
# assert -- `prepared_program_reruns_are_isolated_and_survive_a_trap` measures
# the warm median of a persistent rerun against the 20 ms M9 planning budget
# (the reference host measures 17.4 ms). It is classified in the M19 plan
# (§3.1) as failing under load and passing solo, so it has to be measured on a
# machine that is not still busy with a multi-minute test suite: on the
# independent runner of run #46 it measured 20.83 ms right after the workspace
# suite, and cargo aborts a test invocation at the first failing binary, so
# the self-host suites never ran in that job. The budget constant and the
# assertion are unchanged; only the order is: the timing-sensitive binary runs
# first, on a fresh machine, and the rest of the suite runs after it with the
# timing test skipped so that it is measured exactly once.
Invoke-Step 'test (runner core)' { & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test runner -- --test-threads=1; $LASTEXITCODE }
Invoke-Step 'test (workspace)' { & $cargo test --locked --offline --workspace --all-targets --all-features; $LASTEXITCODE }
Invoke-Step 'test (runner suite)' { & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml -- --test-threads=1 --skip prepared_program_reruns_are_isolated_and_survive_a_trap; $LASTEXITCODE }
Invoke-Step 'module boundaries' { & powershell -ExecutionPolicy Bypass -File tools\validate-module-boundaries.ps1; $LASTEXITCODE }
# The guest frontend's lexer does not skip `//` comments, so a comment inside
# a function body is read as source by the guest's statement scanner and a
# comment word is counted by its structural scans. STEP-0508 hit exactly that
# and regressed the full parser.sico self-host; this step keeps the
# discipline mechanical instead of prose (tools/validate-selfhost-comment-
# discipline.py is read-only).
Invoke-Step 'selfhost comment discipline' { & $env:SICO_PYTHON tools\validate-selfhost-comment-discipline.py; $LASTEXITCODE }
Invoke-Step 'planning contract (M14-M25)' { & powershell -ExecutionPolicy Bypass -File tools\validate-step-0124.ps1; $LASTEXITCODE }
Invoke-Step 'application-profile matrix' { & powershell -ExecutionPolicy Bypass -File tools\validate-step-0131.ps1; $LASTEXITCODE }
Invoke-Step 'cross-host matrix + UI corpus' { & powershell -ExecutionPolicy Bypass -File tools\validate-step-0156.ps1; $LASTEXITCODE }
Invoke-Step 'whitespace (git diff --check)' { git diff --check; $LASTEXITCODE }

if ($failures.Count -gt 0) {
    Write-Host ("CI RED: " + ($failures -join ', ')) -ForegroundColor Red
    # The per-step table also lands in the job summary, which the run page
    # renders without a signed-in session.
    if ($env:GITHUB_STEP_SUMMARY) {
        $summary = @("| step | elapsed | exit |", "|---|---|---|")
        foreach ($row in $results) {
            $parts = $row -split "`t"
            $summary += ("| {0} | {1} | {2} |" -f $parts[0], $parts[1], $parts[2])
        }
        [System.IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, (($summary -join "`n") + "`n"))
    }
    exit 1
}
if ($env:GITHUB_STEP_SUMMARY) {
    $summary = @("| step | elapsed | exit |", "|---|---|---|")
    foreach ($row in $results) {
        $parts = $row -split "`t"
        $summary += ("| {0} | {1} | {2} |" -f $parts[0], $parts[1], $parts[2])
    }
    [System.IO.File]::AppendAllText($env:GITHUB_STEP_SUMMARY, (($summary -join "`n") + "`n"))
}
Write-Host 'CI GREEN' -ForegroundColor Green
exit 0
