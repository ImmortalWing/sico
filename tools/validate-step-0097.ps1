$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path -LiteralPath $vcvars)) {
    throw 'msvc-environment-missing|STEP-0097 requires the native Windows runner toolchain'
}
$runner = Join-Path $root 'runner\sico-runner'
$command = 'call "{0}" && set RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc&& cd /d "{1}" && cargo test --release --offline && cargo clippy --release --offline --all-targets -- -D warnings' -f $vcvars, $runner
& cmd.exe /d /s /c $command
if ($LASTEXITCODE -ne 0) { throw 'native-runner-validation-failed|STEP-0097' }

$cargo = Join-Path $HOME '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
& $cargo test --offline -p sico-observability -p sico-codegen-wasm
if ($LASTEXITCODE -ne 0) { throw 'contract-codegen-regression|STEP-0097' }

$powershell = (Get-Command powershell.exe -ErrorAction Stop).Source
& $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'tools\validate-step-0096.ps1') | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'debug-triplet-regression|STEP-0096 validator' }
& $powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'tools\validate-module-boundaries.ps1') | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'module-boundary-regression|STEP-0097 validator' }

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
