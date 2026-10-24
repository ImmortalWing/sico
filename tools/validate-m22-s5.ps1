<#
.SYNOPSIS
M22 S5 self-hosting validation battery (STEP-0536, owner audit R0.2).

.DESCRIPTION
The versioned, reproducible battery for the M22 S5 wave.  It replaces the
throwaway `target/tmp/stepNNNN/battery.ps1` scripts: every step lives in this
repository, the commands are recorded with their wall time and exit code, and
**any** failing step fails the whole battery.

The battery covers, in order:

  1. rustfmt for the root workspace and for the separate runner workspace;
  2. clippy for both workspaces (deny warnings);
  3. the one-command CI (`tools/run-ci.ps1`);
  4. the STEP-0245 and STEP-0262 validators;
  5. the frontend and IR-plan frontier probes;
  6. the selfhost comment-discipline validator;
  7. the `match` site ledger census check and the per-site coverage probe
     (owner audit R0.4);
  8. the component-layer emitter vocabulary census (STEP-0537): the named
     per-site gap list of the component axis;
  9. the selfhost firmware suite on its own, so the R0 exit evidence is an
     explicit step and not a by-product of the CI step;
 10. `git diff --check`.

Unlike the retired ad-hoc batteries this one carries **no hard-coded dirty
file list**: the workspace hygiene claim is made by the versioned ledger and
the comment-discipline validator, not by a list of paths that has to be edited
whenever a file moves.

Exit codes: 0 = every step exited 0, 1 = at least one step failed.

Usage:
  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5.ps1
  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5.ps1 -SkipCi
#>
[CmdletBinding()]
param(
    [switch]$SkipCi
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $false

$repoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $repoRoot
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { Write-Error 'cargo not found'; exit 1 }
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$env:SICO_SELFHOST_COMPONENT_CACHE = '0'
$env:RUST_TEST_THREADS = '1'
$env:SICO_PYTHON = & (Join-Path $repoRoot 'tools\ensure-python.ps1')
$pythonDir = Split-Path -Parent $env:SICO_PYTHON
$env:Path = "$pythonDir;$env:Path"

$workDir = Join-Path $repoRoot 'target\tmp\m22-s5-battery'
New-Item -ItemType Directory -Force -Path $workDir | Out-Null
$log = Join-Path $workDir 'battery.log'
[System.IO.File]::WriteAllText($log, '')

$results = New-Object System.Collections.ArrayList

function Write-Log([string]$text) {
    [System.IO.File]::AppendAllText($log, $text + "`n", (New-Object System.Text.UTF8Encoding($false)))
}

function Invoke-BatteryStep([string]$name, [string]$command, [scriptblock]$action) {
    Write-Host ("=== {0}" -f $name)
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    $output = & $action 2>&1 | Out-String
    $code = $LASTEXITCODE
    $watch.Stop()
    $seconds = [Math]::Round($watch.Elapsed.TotalSeconds, 0)
    $line = "{0,-26} {1,6}s exit={2}  {3}" -f $name, $seconds, $code, $command
    Write-Host $line
    Write-Log $line
    Write-Log ($output.TrimEnd())
    [void]$results.Add([pscustomobject]@{ Name = $name; Command = $command; Seconds = $seconds; Exit = $code })
}

Invoke-BatteryStep 'fmt (workspace)' 'cargo fmt --all -- --check' {
    & $cargo fmt --all -- --check
}
Invoke-BatteryStep 'fmt (runner)' 'cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check' {
    & $cargo fmt --manifest-path .\runner\sico-runner\Cargo.toml --all -- --check
}
Invoke-BatteryStep 'clippy (workspace)' 'cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings' {
    & $cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
}
Invoke-BatteryStep 'clippy (runner)' 'cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings' {
    & $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings
}
if (-not $SkipCi) {
    Invoke-BatteryStep 'ci-one-key' 'powershell -File tools/run-ci.ps1' {
        & powershell -ExecutionPolicy Bypass -File .\tools\run-ci.ps1
    }
}
Invoke-BatteryStep 'validate-0245' 'powershell -File tools/validate-step-0245.ps1' {
    & powershell -ExecutionPolicy Bypass -File .\tools\validate-step-0245.ps1
}
Invoke-BatteryStep 'validate-0262' 'powershell -File tools/validate-step-0262.ps1' {
    & powershell -ExecutionPolicy Bypass -File .\tools\validate-step-0262.ps1
}
Invoke-BatteryStep 'probe-frontend-frontier' 'powershell -File tools/probe-m22-frontend-frontier.ps1' {
    & powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1
}
Invoke-BatteryStep 'probe-ir-plan-frontier' 'powershell -File tools/probe-m22-ir-plan-frontier.ps1' {
    & powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-ir-plan-frontier.ps1
}
Invoke-BatteryStep 'comment-discipline' 'python tools/validate-selfhost-comment-discipline.py' {
    & $env:SICO_PYTHON .\tools\validate-selfhost-comment-discipline.py
}
Invoke-BatteryStep 'match-site-census' 'python tools/census-m22-match-sites.py --check' {
    & $env:SICO_PYTHON .\tools\census-m22-match-sites.py --check
}
Invoke-BatteryStep 'match-site-coverage' 'powershell -File tools/probe-m22-match-coverage.ps1' {
    & powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-match-coverage.ps1
}
Invoke-BatteryStep 'emitter-vocabulary-census' 'python tools/census-m22-emitter-vocabulary.py --check' {
    & $env:SICO_PYTHON .\tools\census-m22-emitter-vocabulary.py --check
}
Invoke-BatteryStep 'firmware-selfhost' 'cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler' {
    & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --test-threads=1
}
Invoke-BatteryStep 'git-diff-check' 'git diff --check' {
    git diff --check
}

$red = @($results | Where-Object { $_.Exit -ne 0 })
Write-Host ''
foreach ($item in $results) {
    Write-Host ("{0,-26} {1,6}s exit={2}" -f $item.Name, $item.Seconds, $item.Exit)
}
if ($red.Count -eq 0) {
    Write-Host ("BATTERY_GREEN {0}/{0}" -f $results.Count) -ForegroundColor Green
    Write-Host ("log: {0}" -f $log)
    exit 0
}
Write-Host ("BATTERY_RED {0}/{1}" -f $red.Count, $results.Count) -ForegroundColor Red
foreach ($item in $red) {
    Write-Host ("  RED {0} exit={1}  {2}" -f $item.Name, $item.Exit, $item.Command) -ForegroundColor Red
}
Write-Host ("log: {0}" -f $log)
exit 1
