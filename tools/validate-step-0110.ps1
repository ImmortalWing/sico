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

# --- current toolchain hygiene ---
Invoke-NativeChecked $cargo @('fmt', '--all', '--check') 'cargo fmt check failed'
Invoke-NativeChecked $cargo @('clippy', '--workspace', '--all-targets', '--offline', '--locked', '--', '-D', 'warnings') 'clippy failed'
Invoke-NativeChecked $cargo @('test', '--workspace', '--offline', '--locked') 'workspace-tests-failed|STEP-0110'

# --- M0–M10 regression: the M10 exit-audit validator owns that aggregate
# --- (0094 = M0–M9, then 0095–0101) and is rerun, never remembered ---
Invoke-Step '0102'

# --- M11 chain: 0108 gates 0107 (transitively 0106/0105/0104/0091/0100/0090/0087) ---
Invoke-Step '0108'

# --- M11 evidence markers must exist from this machine's runs ---
foreach ($marker in @(
        @{ Path = 'target\evidence\step-0105\runner-tests.txt'; Pattern = 'SCHEDULER_TEARDOWN_100' },
        @{ Path = 'target\evidence\step-0106\runner-tests.txt'; Pattern = 'CHAIN_CANCEL_1024' },
        @{ Path = 'target\evidence\step-0107\runner-tests.txt'; Pattern = 'CHANNEL_RELAY_1GIB' },
        @{ Path = 'target\evidence\step-0108\runner-tests.txt'; Pattern = 'GRANT_MATRIX_100' }
    )) {
    Require-Text $marker.Path $marker.Pattern
}

# --- audit document consistency ---
Require-Text 'docs\adr\ADR-0010-single-store-structured-concurrency.md' 'status: accepted'
Require-Text 'docs\rfc\RFC-0036-structured-concurrency-semantic-ir-v0.md' 'status: accepted'
Require-Text 'docs\steps\STEP-0105-scheduler-core.md' 'status: complete'
Require-Text 'docs\steps\STEP-0106-cancellation-race-select.md' 'status: complete'
Require-Text 'docs\steps\STEP-0107-bounded-channels-streams.md' 'status: complete'
Require-Text 'docs\steps\STEP-0108-persistent-watch-repl-dap-task-integration.md' 'status: complete'
Require-Text 'docs\steps\STEP-0109-cross-platform-runner-parity.md' 'status: complete'
Require-Text 'target\evidence\step-0109\linux\environment.txt' 'microsoft-standard-WSL2'
Require-Text 'docs\steps\STEP-0110-m11-exit-audit.md' 'status: complete / GO'

# --- platform honesty: this audit is Windows-only evidence ---
$rustc = Join-Path $env:USERPROFILE '.cargo\bin\rustc.exe'
$rustcInfo = (Invoke-NativeChecked $rustc @('-vV') 'rustc-host-query-failed' 2>&1) -join "`n"
if ($rustcInfo -notmatch '(?m)^host: x86_64-pc-windows-gnu$') {
    throw 'unsupported-platform-claim|expected actual windows-x64-gnu execution'
}

Write-Output 'STEP_0110_OK m0-m10=green m11=0103-0109-green evidence=present platform=windows-x64+linux-x64 gates=10/10 decision=GO next=M12'
