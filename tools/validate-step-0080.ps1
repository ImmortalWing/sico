param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'

cargo test --offline --locked -p sico-codegen-wasm -p sico-runtime -p sico-cli -p sico-package -p sico-app-cli
if ($LASTEXITCODE -ne 0) { throw 'STEP-0080 workspace tests failed' }

$wasmtime = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
$version = (& $wasmtime --version).Trim()
if ($LASTEXITCODE -ne 0 -or $version -cne 'wasmtime 46.0.1 (823d1b8f2 2026-06-24)') {
    throw "unexpected Wasmtime runtime: $version"
}

$work = Join-Path $root 'target/evidence/step-0080'
New-Item -ItemType Directory -Force -Path $work | Out-Null
$echo = Join-Path $root 'tests/end-to-end/script-echo.sico'
$reject = Join-Path $root 'tests/end-to-end/script-reject.sico'
$echoComponent = Join-Path $work 'echo.component.wasm'
$rejectComponent = Join-Path $work 'reject.component.wasm'
$echoCommand = Join-Path $work 'echo.command.wasm'
$rejectCommand = Join-Path $work 'reject.command.wasm'
$package = Join-Path $work 'echo.sapp'
Get-ChildItem $work -File | Remove-Item -Force

cargo run -q --offline --locked -p sico-cli -- build --profile script-v0 --output $echoComponent $echo
if ($LASTEXITCODE -ne 0) { throw 'script profile build failed (echo)' }
cargo run -q --offline --locked -p sico-cli -- build --profile script-v0 --output $rejectComponent $reject
if ($LASTEXITCODE -ne 0) { throw 'script profile build failed (reject)' }
$composeReport = cargo run -q --offline --locked -p sico-app-cli -- compose $echoComponent -o $echoCommand
if ($LASTEXITCODE -ne 0) { throw 'adapter composition failed (echo)' }
cargo run -q --offline --locked -p sico-app-cli -- compose $rejectComponent -o $rejectCommand | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'adapter composition failed (reject)' }

$payload = 'step-0080-payload'
$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
$stdout = $payload | & $wasmtime run $echoCommand alpha beta 2>$null
$echoExit = $LASTEXITCODE
$stderr = $payload | & $wasmtime run $echoCommand alpha beta 2>&1>$null
$null = $payload | & $wasmtime run $rejectCommand 2>$work\reject-stderr.txt
$rejectExit = $LASTEXITCODE
$ErrorActionPreference = $previousPreference
if ($echoExit -ne 0 -or ($stdout -join '') -cne $payload) {
    throw "echo command mismatch: exit=$echoExit stdout=$stdout"
}
if ($rejectExit -ne 1 -or -not ((Get-Content $work\reject-stderr.txt -Raw) -cmatch 'rejected')) {
    throw "reject command mismatch: exit=$rejectExit"
}

cargo run -q --offline --locked -p sico-app-cli -- pack --script --output $package $echoCommand | Out-Null
if ($LASTEXITCODE -ne 0) { throw 'script pack failed' }
$inspect = cargo run -q --offline --locked -p sico-app-cli -- inspect --json $package
if ($LASTEXITCODE -ne 0) { throw 'script inspect failed' }
$manifest = ($inspect | ConvertFrom-Json)
if ($manifest.manifest_schema -cne 'sico.sapp.manifest.v1') { throw 'manifest is not v1' }
if ($manifest.script.world -cne 'sico:script/program@0.1.0') { throw 'wrong script world' }
if ($manifest.script.adapter.id -cne 'sico:script/adapter@0.1.0') { throw 'wrong adapter id' }

$adapterLine = ($composeReport | Where-Object { $_ -match '^\{' })
$adapter = ($adapterLine | ConvertFrom-Json).adapter
Write-Output "STEP_0080_OK runtime=$version adapter=$($adapter.id)@$($adapter.sha256.Substring(0,16)) composed=echo+reject exit=0/1 manifest=v1 closure=script.args,script.stdio work=$work"
