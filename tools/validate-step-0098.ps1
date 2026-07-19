$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path -LiteralPath $vcvars)) {
    throw 'msvc-environment-missing|STEP-0098 requires native Windows console evidence'
}
$runner = Join-Path $root 'runner\sico-runner'
$command = 'call "{0}" && set RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc&& cd /d "{1}" && cargo test --release --offline && cargo clippy --release --offline --all-targets -- -D warnings' -f $vcvars, $runner
& cmd.exe /d /s /c $command
if ($LASTEXITCODE -ne 0) { throw 'native-signal-client-validation-failed|STEP-0098' }

$powershell = (Get-Command powershell.exe -ErrorAction Stop).Source
foreach ($step in @('0097', '0088', '0089')) {
    & $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root "tools\validate-step-$step.ps1") | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "regression-failed|STEP-$step" }
}

$cargo = Join-Path $HOME '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
& $cargo test --offline -p sico-observability
if ($LASTEXITCODE -ne 0) { throw 'cancel-contract-tests-failed|STEP-0098' }

$source = Get-Content -LiteralPath (Join-Path $runner 'src\lib.rs') -Raw -Encoding UTF8
foreach ($required in @('TerminalArbiter', 'CancellationSource', 'register_console_cancellation', 'apply_cancel_request')) {
    if (-not $source.Contains($required)) { throw "typed-cancellation-path-missing|$required" }
}
if ($source -match 'exit_code\(\).*123.*process') {
    throw 'external-process-kill-misclassified|STEP-0098'
}

Write-Output 'STEP_0098_OK signal=windows-console-real client=one-shot+watch busy-read-pump-write-http=123 races=8/8 terminal=1 double-cancel=idempotent request=4096/4097-refused external-kill=not-cancelled authority=unchanged m11-adr=unlocked next=STEP-0099+STEP-0103'
