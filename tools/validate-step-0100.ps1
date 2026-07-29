$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$env:CARGO_REGISTRIES_CRATES_IO_PROTOCOL = 'sparse'
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
. (Join-Path $root 'tools\lib\native-command.ps1')

Push-Location $root
try {
    Invoke-NativeChecked $cargo @(
        'test', '--offline', '--locked', '-p', 'sico-tooling-protocol'
    ) 'dap-contract-tests-failed|STEP-0100'
    Invoke-NativeChecked $cargo @(
        'test', '--offline', '--locked', '--manifest-path',
        'runner\sico-runner\Cargo.toml', '--lib'
    ) 'runtime-debug-hook-tests-failed|STEP-0100'
    Invoke-NativeChecked $cargo @(
        'test', '--offline', '--locked', '--manifest-path',
        'runner\sico-runner\Cargo.toml', '--test', 'runner', 'dap_', '--',
        '--test-threads=1'
    ) 'real-component-dap-tests-failed|STEP-0100'

    $claims = (Get-Content observability\contracts\dap-claimed-subset-v0.json -Raw -Encoding UTF8 | ConvertFrom-Json).claims
    $supportedRequests = @($claims | Where-Object { $_.kind -eq 'request' -and $_.state -eq 'supported' })
    $refusedRequests = @($claims | Where-Object { $_.kind -eq 'request' -and $_.state -eq 'refused' })
    $supportedEvents = @($claims | Where-Object { $_.kind -eq 'event' -and $_.state -eq 'supported' })
    if ($supportedRequests.Count -ne 12) { throw "supported-request-count|$($supportedRequests.Count)" }
    if ($refusedRequests.Count -ne 20) { throw "refused-request-count|$($refusedRequests.Count)" }
    if ($supportedEvents.Count -ne 6) { throw "supported-event-count|$($supportedEvents.Count)" }

    $runnerManifest = Get-Content runner\sico-runner\Cargo.toml -Raw -Encoding UTF8
    if (-not $runnerManifest.Contains('wasmtime = { version = "=47.0.2"')) {
        throw 'wasmtime-unaligned-frame-fix-not-pinned|STEP-0100'
    }
    foreach ($path in @(
        'runner\sico-runner\src\dap.rs',
        'runner\sico-runner\src\bin\sico-dap.rs',
        'crates\sico-tooling-protocol\src\lib.rs'
    )) {
        if (-not (Test-Path -LiteralPath $path)) { throw "dap-implementation-missing|$path" }
    }
} finally {
    Pop-Location
}

Write-Output 'STEP_0100_OK dap=requests:12/refused:20/events:6 frame=1048576 real=entry,breakpoint,pause,nested-stack,scalar-locals,output,terminal stdio=bounded identity=sha256 teardown=owned wasmtime=47.0.2 platform=windows-x64 next=STEP-0101'
