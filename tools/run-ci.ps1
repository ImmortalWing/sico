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
function Invoke-Step([string]$name, [scriptblock]$body) {
    Write-Host "=== CI: $name" -ForegroundColor Cyan
    $output = & $body 2>&1 | Out-String
    $code = $LASTEXITCODE
    if ($code -ne 0) {
        $output.Substring([Math]::Max(0, $output.Length - 2000)) | Write-Host
        $failures.Add($name) | Out-Null
        Write-Host "=== FAIL: $name" -ForegroundColor Red
    } else {
        Write-Host "=== PASS: $name" -ForegroundColor Green
    }
}

Invoke-Step 'fmt' { & $cargo fmt --all -- --check; $LASTEXITCODE }
if (-not $Fast) {
    Invoke-Step 'clippy (workspace)' { & $cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings; $LASTEXITCODE }
    Invoke-Step 'clippy (runner)' { & $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings; $LASTEXITCODE }
}
Invoke-Step 'build (runner debug)' { & $cargo build --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml; $LASTEXITCODE }
Invoke-Step 'test (workspace)' { & $cargo test --locked --offline --workspace --all-targets --all-features; $LASTEXITCODE }
# The runner suite's RSS/handle budget asserts measure the *process-wide*
# metrics of the test binary; under the default threaded harness, sibling
# test allocations land inside those measurements (deterministic 83 MB
# false growth on this host). The suite is seconds long, so CI runs it
# serially; the budget constants themselves are unchanged.
Invoke-Step 'test (runner)' { & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml -- --test-threads=1; $LASTEXITCODE }
Invoke-Step 'module boundaries' { & powershell -ExecutionPolicy Bypass -File tools\validate-module-boundaries.ps1; $LASTEXITCODE }
Invoke-Step 'planning contract (M14-M25)' { & powershell -ExecutionPolicy Bypass -File tools\validate-step-0124.ps1; $LASTEXITCODE }
Invoke-Step 'application-profile matrix' { & powershell -ExecutionPolicy Bypass -File tools\validate-step-0131.ps1; $LASTEXITCODE }
Invoke-Step 'cross-host matrix + UI corpus' { & powershell -ExecutionPolicy Bypass -File tools\validate-step-0156.ps1; $LASTEXITCODE }
Invoke-Step 'whitespace (git diff --check)' { git diff --check; $LASTEXITCODE }

if ($failures.Count -gt 0) {
    Write-Host ("CI RED: " + ($failures -join ', ')) -ForegroundColor Red
    exit 1
}
Write-Host 'CI GREEN' -ForegroundColor Green
exit 0
