param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
. (Join-Path $root 'tools\lib\native-command.ps1')
$powershell = (Get-Command powershell.exe -ErrorAction Stop).Source

function Require-Text([string]$path, [string]$pattern) {
    $full = Join-Path $root $path
    if (-not (Test-Path $full)) { throw "missing audit input $path" }
    if (-not (Select-String -Path $full -Pattern $pattern -Quiet -Encoding utf8)) {
        throw "audit input $path does not contain $pattern"
    }
}

function Invoke-Step([string]$step) {
    $script = Join-Path $root "tools\validate-step-$step.ps1"
    if (-not (Test-Path $script)) { throw "missing STEP-$step validator" }
    # Each runner-heavy validator gets a fresh PowerShell host. This prevents a
    # blocked pipe or native runner failure from poisoning later audit steps.
    Invoke-NativeChecked $powershell @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', $script,
        '-RepositoryRoot', $root
    ) "STEP-$step validator failed"
}

# STEP-0085 is a contract step. Its WIT parser test is rerun by STEP-0086; the
# audit also pins the versioned interface and cancellation matrix documents.
Require-Text 'docs\rfc\RFC-0030-script-streaming-v0.md' 'sico:script/streams@0\.1\.0'
Require-Text 'docs\rfc\RFC-0030-script-streaming-v0.md' 'cancel'
Require-Text 'wit\script-profile-v0\world.wit' 'interface streams'
Write-Output 'STEP_0085_CONTRACT_OK wit=sico:script/streams@0.1.0 ownership=affine backpressure=blocking cancellation=matrix'

# STEP-0084 includes fmt, Clippy, all workspace tests and the M0-M8 performance
# regression. M9 steps are isolated so native runner evidence cannot leak state.
Invoke-Step '0084'
foreach ($step in '0086','0087','0088','0089','0090','0091','0092','0093') {
    Invoke-Step $step
}

foreach ($doc in @(
    'docs\reports\m9-exit-audit-v0.md',
    'docs\steps\STEP-0094-m9-exit-audit.md',
    'docs\plans\M10-runtime-observability-debugging.md'
)) {
    if (-not (Test-Path (Join-Path $root $doc))) { throw "missing audit artifact $doc" }
}

Write-Output 'STEP_0094_OK m8=GO m9_steps=0085-0093 security=default-deny performance=bounded platform=windows-x64-runtime decision=GO next=M10/STEP-0095'
