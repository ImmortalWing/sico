$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$runner = Join-Path $root 'runner\sico-runner'
$runnerManifest = Join-Path $runner 'Cargo.toml'
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
. (Join-Path $root 'tools\lib\native-command.ps1')
Invoke-NativeChecked $cargo @(
    'test', '--release', '--offline', '--locked', '--manifest-path', $runnerManifest,
    '--', '--test-threads=1'
) 'native-signal-client-tests-failed|STEP-0098'
Invoke-NativeChecked $cargo @(
    'clippy', '--release', '--offline', '--locked', '--manifest-path', $runnerManifest,
    '--all-targets', '--', '-D', 'warnings'
) 'native-signal-client-clippy-failed|STEP-0098'

$powershell = (Get-Command powershell.exe -ErrorAction Stop).Source
foreach ($step in @('0097', '0088', '0089')) {
    Invoke-NativeChecked $powershell @(
        '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
        (Join-Path $root "tools\validate-step-$step.ps1")
    ) "regression-failed|STEP-$step" | Out-Null
}

$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
Invoke-NativeChecked $cargo @(
    'test', '--offline', '-p', 'sico-observability'
) 'cancel-contract-tests-failed|STEP-0098'

$source = Get-Content -LiteralPath (Join-Path $runner 'src\lib.rs') -Raw -Encoding UTF8
foreach ($required in @('TerminalArbiter', 'CancellationSource', 'register_console_cancellation', 'apply_cancel_request')) {
    if (-not $source.Contains($required)) { throw "typed-cancellation-path-missing|$required" }
}
if ($source -match 'exit_code\(\).*123.*process') {
    throw 'external-process-kill-misclassified|STEP-0098'
}

Write-Output 'STEP_0098_OK signal=windows-console-real client=one-shot+watch busy-read-pump-write-http=123 races=8/8 terminal=1 double-cancel=idempotent request=4096/4097-refused external-kill=not-cancelled authority=unchanged m11-adr=unlocked next=STEP-0099+STEP-0103'
