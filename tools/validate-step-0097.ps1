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
) 'native-runner-tests-failed|STEP-0097'
Invoke-NativeChecked $cargo @(
    'clippy', '--release', '--offline', '--locked', '--manifest-path', $runnerManifest,
    '--all-targets', '--', '-D', 'warnings'
) 'native-runner-clippy-failed|STEP-0097'

$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
Invoke-NativeChecked $cargo @(
    'test', '--offline', '-p', 'sico-observability', '-p', 'sico-codegen-wasm'
) 'contract-codegen-regression|STEP-0097'

$powershell = (Get-Command powershell.exe -ErrorAction Stop).Source
Invoke-NativeChecked $powershell @(
    '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
    (Join-Path $root 'tools/validate-step-0096.ps1')
) 'debug-triplet-regression|STEP-0096 validator' | Out-Null
Invoke-NativeChecked $powershell @(
    '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
    (Join-Path $root 'tools/validate-module-boundaries.ps1')
) 'module-boundary-regression|STEP-0097 validator' | Out-Null

$runnerSource = Get-Content -LiteralPath (Join-Path $runner 'src\lib.rs') -Raw -Encoding UTF8
if ($runnerSource -notmatch 'downcast_ref::<wasmtime::WasmBacktrace>') {
    throw 'typed-backtrace-missing|STEP-0097'
}
if ($runnerSource -match 'contains\([^\r\n]*(trap|fuel|timeout|cancel)') {
    throw 'engine-prose-classification-detected|STEP-0097'
}
$mainSource = Get-Content -LiteralPath (Join-Path $runner 'src\main.rs') -Raw -Encoding UTF8
foreach ($required in @('--debug-map', '--debug-identity', 'report_observed', 'canonical_json')) {
    if (-not $mainSource.Contains($required)) { throw "cli-observation-missing|$required" }
}

Write-Output 'STEP_0097_OK classes=9 exact_source=true stale_refusal=true missing_map=unavailable frames=256/257-refused provider=typed cli=json+text engine_text_classification=false authority=unchanged next=STEP-0098'
