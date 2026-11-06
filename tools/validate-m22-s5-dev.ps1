<#
.SYNOPSIS
M22 S5 development-tier battery (STEP-0565, owner decision AGENTS.md 10.1.2).

.DESCRIPTION
Layered subset of `tools/validate-m22-s5.ps1`. The owner decision splits the
single 15-step / ~2.7 h exit battery into four tiers that run during
development:

  fast     ~1 min    hygiene + the cheap censuses
  probes   ~12 min   + the frontier / coverage probes (the measurement tier)
  firmware ~28 min   + the selfhost firmware suite (`selfhost_compiler`)
  all      ~2.7 h    + the one-command CI and the two STEP validators
                     (the full exit battery, tier by tier)

Every step is the SAME command, in the SAME order, as the corresponding step
of `tools/validate-m22-s5.ps1`; this script only changes the rhythm, never the
criteria (AGENTS.md 10.1.2: "分层只改节奏，不降判据"). A dev-tier green is NOT
exit evidence and closes no gate.

Resume: state is written per tier to `target/tmp/m22-s5-dev/<set>.state.json`
after every step. Re-running the same tier skips steps that already exited 0.
The state carries the tree fingerprint from `tools/m22-s5-fingerprint.ps1`
(`target`, `__pycache__`, `*.pyc` excluded = the STEP-0545 6.1 self-pollution
fix). A fingerprint change REFUSES to reuse the old green: the run aborts and
tells you to re-run with `-Fresh`.

Single instance: two concurrent batteries contend for the same `target/` and
the same logs, and such a run's green is not citable (handoff-m22, 2026-10-24).
A PID lock under `target/tmp/m22-s5-dev/` fails the run when another instance
of THIS script holds it.

Usage:
  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5-dev.ps1 -Set fast
  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5-dev.ps1 -Set probes
  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5-dev.ps1 -Set firmware
  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5-dev.ps1 -Set all
  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5-dev.ps1 -Set fast -Fresh
  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5-dev.ps1 -List

Exit codes: 0 every step of the tier exited 0, 1 at least one step failed,
2 usage / lock / fingerprint-gate refusal.
#>
[CmdletBinding()]
param(
    [ValidateSet('fast', 'probes', 'firmware', 'all')]
    [string]$Set = 'fast',
    [switch]$Fresh,
    [switch]$List
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $false

$repoRoot = Split-Path -Parent $PSScriptRoot
Set-Location $repoRoot
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { Write-Error 'cargo not found'; exit 2 }

$workDir = Join-Path $repoRoot 'target\tmp\m22-s5-dev'
$fingerprintScript = Join-Path $repoRoot 'tools\m22-s5-fingerprint.ps1'

# The step catalog. Names, commands and order match tools/validate-m22-s5.ps1
# step for step; a tier is a contiguous slice of this catalog.
$script:catalog = @(
    @{ Name = 'fmt (workspace)';            Command = 'cargo fmt --all -- --check'; Set = 'fast' },
    @{ Name = 'fmt (runner)';               Command = 'cargo fmt --manifest-path runner/sico-runner/Cargo.toml --all -- --check'; Set = 'fast' },
    @{ Name = 'clippy (workspace)';         Command = 'cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings'; Set = 'all' },
    @{ Name = 'clippy (runner)';            Command = 'cargo clippy --locked --offline --manifest-path runner/sico-runner/Cargo.toml --all-targets -- -D warnings'; Set = 'all' },
    @{ Name = 'ci-one-key';                 Command = 'powershell -File tools/run-ci.ps1'; Set = 'all' },
    @{ Name = 'validate-0245';              Command = 'powershell -File tools/validate-step-0245.ps1'; Set = 'all' },
    @{ Name = 'validate-0262';              Command = 'powershell -File tools/validate-step-0262.ps1'; Set = 'all' },
    @{ Name = 'probe-frontend-frontier';    Command = 'powershell -File tools/probe-m22-frontend-frontier.ps1'; Set = 'probes' },
    @{ Name = 'probe-ir-plan-frontier';     Command = 'powershell -File tools/probe-m22-ir-plan-frontier.ps1'; Set = 'probes' },
    @{ Name = 'comment-discipline';         Command = 'python tools/validate-selfhost-comment-discipline.py'; Set = 'fast' },
    @{ Name = 'match-site-census';          Command = 'python tools/census-m22-match-sites.py --check'; Set = 'fast' },
    @{ Name = 'match-site-coverage';        Command = 'powershell -File tools/probe-m22-match-coverage.ps1'; Set = 'probes' },
    @{ Name = 'emitter-vocabulary-census';  Command = 'python tools/census-m22-emitter-vocabulary.py --check'; Set = 'fast' },
    @{ Name = 'firmware-selfhost';          Command = 'cargo test --manifest-path runner/sico-runner/Cargo.toml --test selfhost_compiler'; Set = 'firmware' },
    @{ Name = 'git-diff-check';             Command = 'git diff --check'; Set = 'fast' }
)

function Get-SetSteps([string]$setName) {
    if ($setName -eq 'all') { return $script:catalog }
    return @($script:catalog | Where-Object { $_.Set -eq $setName })
}

if ($List) {
    foreach ($tier in @('fast', 'probes', 'firmware', 'all')) {
        $steps = Get-SetSteps $tier
        Write-Host ("{0,-9} {1,2} steps" -f $tier, $steps.Count)
        foreach ($step in $steps) { Write-Host ("  {0,-26} {1}" -f $step.Name, $step.Command) }
    }
    exit 0
}

$steps = Get-SetSteps $Set
if ($steps.Count -eq 0) { Write-Error ("unknown set: {0}" -f $Set); exit 2 }

New-Item -ItemType Directory -Force -Path $workDir | Out-Null

# --- single-instance lock -------------------------------------------------
$lockPath = Join-Path $workDir 'battery.lock'
if (Test-Path -LiteralPath $lockPath -PathType Leaf) {
    $existingPid = 0
    [void][int]::TryParse((Get-Content -LiteralPath $lockPath -Raw), [ref]$existingPid)
    $alive = $false
    if ($existingPid -gt 0 -and $existingPid -ne $PID) {
        $alive = $null -ne (Get-Process -Id $existingPid -ErrorAction SilentlyContinue)
    }
    if ($alive) {
        Write-Host ("DEV_BATTERY_LOCKED pid={0} lock={1}" -f $existingPid, $lockPath) -ForegroundColor Red
        Write-Host 'another instance of validate-m22-s5-dev.ps1 is running; terminate it before starting one (single-instance rule, handoff-m22 2026-10-24).'
        exit 2
    }
}
[System.IO.File]::WriteAllText($lockPath, "$PID`n")

# --- environment (same as tools/validate-m22-s5.ps1) ----------------------
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$env:SICO_SELFHOST_COMPONENT_CACHE = '0'
$env:RUST_TEST_THREADS = '1'
$env:SICO_PYTHON = & (Join-Path $repoRoot 'tools\ensure-python.ps1')
$pythonDir = Split-Path -Parent $env:SICO_PYTHON
$env:Path = "$pythonDir;$env:Path"

# --- fingerprint gate -----------------------------------------------------
$fingerprintLine = & powershell -NoProfile -ExecutionPolicy Bypass -File $fingerprintScript
$fingerprintExit = $LASTEXITCODE
if ($fingerprintExit -ne 0) {
    Write-Host "DEV_BATTERY_FINGERPRINT_ERROR" -ForegroundColor Red
    Write-Host $fingerprintLine
    Remove-Item -LiteralPath $lockPath -Force
    exit 2
}
$fingerprint = ($fingerprintLine | Select-String -Pattern 'FINGERPRINT\s+([0-9a-f]{64})\s+files=(\d+)').Matches[0]
$treeFingerprint = $fingerprint.Groups[1].Value
$treeFileCount = [int]$fingerprint.Groups[2].Value
$statePath = Join-Path $workDir ("{0}.state.json" -f $Set)
$log = Join-Path $workDir ("{0}.log" -f $Set)

if ($Fresh -and (Test-Path -LiteralPath $statePath -PathType Leaf)) {
    Remove-Item -LiteralPath $statePath -Force
}

$state = $null
if (Test-Path -LiteralPath $statePath -PathType Leaf) {
    $state = Get-Content -LiteralPath $statePath -Raw | ConvertFrom-Json
    if ($state.fingerprint -ne $treeFingerprint) {
        Write-Host "DEV_BATTERY_STATE_STALE" -ForegroundColor Red
        Write-Host ("  tier            = {0}" -f $Set)
        Write-Host ("  state file      = {0}" -f $statePath)
        Write-Host ("  state fingerprint = {0} (files={1})" -f $state.fingerprint, $state.fileCount)
        Write-Host ("  tree fingerprint  = {0} (files={1})" -f $treeFingerprint, $treeFileCount)
        Write-Host 'the tree changed since this state was written; the old green is not reusable.'
        Write-Host ("  re-run with -Fresh to discard it:  powershell -ExecutionPolicy Bypass -File tools/validate-m22-s5-dev.ps1 -Set {0} -Fresh" -f $Set)
        Remove-Item -LiteralPath $lockPath -Force
        exit 2
    }
}

if ($null -eq $state) {
    $state = [pscustomobject]@{
        schema      = 'sico.m22.s5.dev-battery-state.v1'
        set         = $Set
        fingerprint = $treeFingerprint
        fileCount   = $treeFileCount
        startedAt   = [DateTime]::Now.ToString('o')
        steps       = New-Object System.Collections.ArrayList
    }
}

$done = @{}
foreach ($record in $state.steps) { $done[$record.Name] = $record }
$alreadyGreen = @($done.Values | Where-Object { $_.Exit -eq 0 }).Count

[System.IO.File]::WriteAllText($log, '', (New-Object System.Text.UTF8Encoding($false)))

function Write-Log([string]$text) {
    [System.IO.File]::AppendAllText($log, $text + "`n", (New-Object System.Text.UTF8Encoding($false)))
}

function Save-State {
    $state.startedAt = $state.startedAt
    $json = $state | ConvertTo-Json -Depth 6
    $temp = "$statePath.tmp"
    [System.IO.File]::WriteAllText($temp, $json + "`n", (New-Object System.Text.UTF8Encoding($false)))
    Move-Item -Force -Path $temp -Destination $statePath
}

function Invoke-DevStep([hashtable]$step) {
    $name = $step.Name
    if ($done.ContainsKey($name) -and $done[$name].Exit -eq 0) {
        $record = $done[$name]
        Write-Host ("=== {0} (skipped, exit=0 {1}s from state)" -f $name, $record.Seconds)
        Write-Log ("{0,-26} SKIPPED exit=0  {1}" -f $name, $step.Command)
        return
    }
    Write-Host ("=== {0}" -f $name)
    $watch = [System.Diagnostics.Stopwatch]::StartNew()
    switch ($name) {
        'fmt (workspace)'         { & $cargo fmt --all -- --check }
        'fmt (runner)'            { & $cargo fmt --manifest-path .\runner\sico-runner\Cargo.toml --all -- --check }
        'clippy (workspace)'      { & $cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings }
        'clippy (runner)'         { & $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings }
        'ci-one-key'              { & powershell -ExecutionPolicy Bypass -File .\tools\run-ci.ps1 }
        'validate-0245'           { & powershell -ExecutionPolicy Bypass -File .\tools\validate-step-0245.ps1 }
        'validate-0262'           { & powershell -ExecutionPolicy Bypass -File .\tools\validate-step-0262.ps1 }
        'probe-frontend-frontier' { & powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-frontend-frontier.ps1 }
        'probe-ir-plan-frontier'  { & powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-ir-plan-frontier.ps1 }
        'comment-discipline'      { & $env:SICO_PYTHON .\tools\validate-selfhost-comment-discipline.py }
        'match-site-census'       { & $env:SICO_PYTHON .\tools\census-m22-match-sites.py --check }
        'match-site-coverage'     { & powershell -ExecutionPolicy Bypass -File .\tools\probe-m22-match-coverage.ps1 }
        'emitter-vocabulary-census'{ & $env:SICO_PYTHON .\tools\census-m22-emitter-vocabulary.py --check }
        'firmware-selfhost'       { & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --test-threads=1 }
        'git-diff-check'          { git diff --check }
        default                   { Write-Error ("no action wired for step {0}" -f $name); $global:LASTEXITCODE = 1 }
    }
    $code = $LASTEXITCODE
    $watch.Stop()
    $seconds = [Math]::Round($watch.Elapsed.TotalSeconds, 0)
    $line = "{0,-26} {1,6}s exit={2}  {3}" -f $name, $seconds, $code, $step.Command
    Write-Host $line
    Write-Log $line
    $record = [pscustomobject]@{ Name = $name; Command = $step.Command; Seconds = $seconds; Exit = $code; FinishedAt = [DateTime]::Now.ToString('o') }
    $kept = New-Object System.Collections.ArrayList
    foreach ($existing in $state.steps) { if ($existing.Name -ne $name) { [void]$kept.Add($existing) } }
    $state.steps = $kept
    [void]$state.steps.Add($record)
    $done[$name] = $record
    Save-State
}

Write-Host ("DEV_BATTERY_START set={0} steps={1} fingerprint={2} files={3}" -f $Set, $steps.Count, $treeFingerprint, $treeFileCount)
Write-Host ("  state={0}" -f $statePath)
if ($alreadyGreen -gt 0) {
    Write-Host ("  resuming: {0}/{1} step(s) already green in state" -f $alreadyGreen, $steps.Count)
}

foreach ($step in $steps) { Invoke-DevStep ([hashtable]$step) }

$red = @()
foreach ($step in $steps) {
    $record = $done[$step.Name]
    if ($null -eq $record -or $record.Exit -ne 0) { $red += $step.Name }
}

Write-Host ''
foreach ($step in $steps) {
    $record = $done[$step.Name]
    if ($null -eq $record) {
        Write-Host ("{0,-26} {1,6}s exit=-" -f $step.Name, 0)
    } else {
        Write-Host ("{0,-26} {1,6}s exit={2}" -f $record.Name, $record.Seconds, $record.Exit)
    }
}

Remove-Item -LiteralPath $lockPath -Force -ErrorAction SilentlyContinue

if ($red.Count -eq 0) {
    Write-Host ("DEV_SUBSET_GREEN {0}/{0} set={1} fingerprint={2}" -f $steps.Count, $Set, $treeFingerprint) -ForegroundColor Green
    Write-Host "  this is a development-tier subset: it is NOT exit evidence and closes no gate (AGENTS.md 10.1.2)."
    Write-Host ("  log: {0}" -f $log)
    exit 0
}

Write-Host ("DEV_SUBSET_RED {0}/{1} set={2} fingerprint={3}" -f $red.Count, $steps.Count, $Set, $treeFingerprint) -ForegroundColor Red
foreach ($name in $red) {
    $record = $done[$name]
    $exit = if ($null -eq $record) { '-' } else { $record.Exit }
    Write-Host ("  RED {0} exit={1}  {2}" -f $name, $exit, ($steps | Where-Object { $_.Name -eq $name }).Command) -ForegroundColor Red
}
Write-Host ("  log: {0}" -f $log)
exit 1
