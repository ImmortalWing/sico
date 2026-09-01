$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $HOME '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$env:CARGO_REGISTRIES_CRATES_IO_PROTOCOL = 'sparse'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
. (Join-Path $root 'tools\lib\native-command.ps1')

Push-Location $root
try {
    Invoke-NativeChecked $cargo @(
        'test', '--offline', '--locked', '-p', 'sico-observability'
    ) 'execution-event-contract-tests-failed|STEP-0099'
    Invoke-NativeChecked $cargo @(
        'test', '--offline', '--locked', '--manifest-path',
        'runner\sico-runner\Cargo.toml', '--lib'
    ) 'runner-event-producer-tests-failed|STEP-0099'

    $contract = Get-Content crates\sico-observability\src\lib.rs -Raw -Encoding UTF8
    foreach ($required in @('ExecutionEvent', 'EventQueue', 'EventRedactor', 'MAX_EVENT_QUEUE_ITEMS', 'MAX_EVENT_QUEUE_BYTES', 'MAX_CAPTURED_CHANNEL_BYTES')) {
        if (-not $contract.Contains($required)) { throw "event-contract-missing|$required" }
    }
    $runner = Get-Content runner\sico-runner\src\lib.rs -Raw -Encoding UTF8
    foreach ($required in @('execution_events', 'cancellation-requested', 'truncated', 'terminal')) {
        if (-not $runner.Contains($required)) { throw "runner-event-producer-missing|$required" }
    }
} finally {
    Pop-Location
}

Write-Output 'STEP_0099_OK schema=sico.execution-event.v0 chunk=65536 capture=1048576 queue=256/4194304 binary=base64 redaction=mandatory sequence=monotonic terminal=reserved next=STEP-0100'
