param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
. (Join-Path $root 'tools\lib\native-command.ps1')
$powershell = (Get-Command powershell.exe -ErrorAction Stop).Source

function Invoke-Step([string]$Step) {
    $script = Join-Path $root "tools\validate-step-$Step.ps1"
    if (-not (Test-Path -LiteralPath $script)) { throw "missing-validator|STEP-$Step" }
    Invoke-NativeChecked $powershell @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $script,
        '-RepositoryRoot', $root
    ) "validator-failed|STEP-$Step"
}

function Require-Text([string]$RelativePath, [string]$Pattern) {
    $path = Join-Path $root $RelativePath
    if (-not (Test-Path -LiteralPath $path)) { throw "missing-audit-input|$RelativePath" }
    if (-not (Select-String -LiteralPath $path -Pattern $Pattern -Quiet -Encoding UTF8)) {
        throw "audit-input-mismatch|$RelativePath|$Pattern"
    }
}

# STEP-0094 owns the complete M0-M9 aggregate. M10 validators are then rerun
# in dependency order so this exit result never relies on remembered output.
Invoke-Step '0094'
foreach ($step in '0095','0096','0097','0098','0099','0100','0101') {
    Invoke-Step $step
}

$claimsPath = Join-Path $root 'observability\contracts\dap-claimed-subset-v0.json'
$claims = (Get-Content -LiteralPath $claimsPath -Raw -Encoding UTF8 | ConvertFrom-Json).claims
$supportedRequests = @($claims | Where-Object { $_.kind -eq 'request' -and $_.state -eq 'supported' }).Count
$refusedRequests = @($claims | Where-Object { $_.kind -eq 'request' -and $_.state -eq 'refused' }).Count
$supportedEvents = @($claims | Where-Object { $_.kind -eq 'event' -and $_.state -eq 'supported' }).Count
if ($supportedRequests -ne 12 -or $refusedRequests -ne 20 -or $supportedEvents -ne 6) {
    throw "dap-claim-count-mismatch|$supportedRequests/$refusedRequests/$supportedEvents"
}

$rustc = Join-Path $env:USERPROFILE '.cargo\bin\rustc.exe'
if (-not (Test-Path -LiteralPath $rustc)) { $rustc = (Get-Command rustc -ErrorAction Stop).Source }
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$rustcInfo = (Invoke-NativeChecked $rustc @('-vV') 'rustc-host-query-failed' 2>&1) -join "`n"
if ($rustcInfo -notmatch '(?m)^host: x86_64-pc-windows-gnu$') {
    throw 'unsupported-platform-claim|expected actual windows-x64-gnu execution'
}

$benchmarkPath = Join-Path $root 'target\evidence\step-0084\benchmark.json'
$benchmark = Get-Content -LiteralPath $benchmarkPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ([int]$benchmark.iterations -lt 20) { throw 'benchmark-sample-count-too-small' }
$coldGoal = if ([double]$benchmark.sico_run_cold_cache_miss_ms.p95 -lt 200) { 'met' } else { 'miss' }
$warmGoal = if ([double]$benchmark.sico_run_warm_cache_hit_ms.p95 -lt 120) { 'met' } else { 'miss' }

Require-Text 'docs\reports\m10-exit-audit-v0.md' 'decision: GO'
Require-Text 'docs\reports\m10-exit-audit-v0.md' 'Windows x64 GNU'
Require-Text 'docs\reports\m10-exit-audit-v0.md' 'blocked-external-evidence'
Require-Text 'docs\steps\STEP-0102-m10-exit-audit.md' 'complete / GO'
Require-Text 'docs\adr\ADR-0010-single-store-structured-concurrency.md' 'status: accepted'
Require-Text 'docs\plans\M11-structured-concurrency-runtime.md' 'STEP-0103'
Require-Text 'docs\reports\project-completion-audit.md' 'blocked-external-evidence'
Require-Text 'docs\reports\ai-tooling-measured-evaluation-v0.md' 'live-model-not-authorized'
Require-Text 'docs\steps\STEP-0069-third-party-pilot-m7-exit.md' 'blocked-external-evidence'
Require-Text 'docs\steps\STEP-0060-desktop-android-parity-app.md' 'partial-runtime-evidence'

$summary = (
    'STEP_0102_OK m0-m9=green m10=0095-0101-green security=fail-closed dap={0}/{1}/{2} ' +
    'platform=windows-x64-gnu powershell={3} benchmark=20+ cold-goal={4} warm-goal={5} ' +
    'external=unchanged-blocked decision=GO next=M11/STEP-0103'
) -f $supportedRequests, $refusedRequests, $supportedEvents, $PSVersionTable.PSVersion.Major, $coldGoal, $warmGoal
Write-Output $summary
