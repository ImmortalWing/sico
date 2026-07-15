[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$cargoBin = Join-Path $env:USERPROFILE '.cargo\bin'
$env:Path = "$cargoBin;$env:Path"

$core = Join-Path $root 'guest\target\wasm32-unknown-unknown\release\sico_component_guest.wasm'
$component = Join-Path $root 'artifacts\sico-component.component.wasm'
$asyncCore = Join-Path $root 'async-guest\target\wasm32-unknown-unknown\release\sico_component_async_guest.wasm'
$asyncComponent = Join-Path $root 'artifacts\sico-component-async.component.wasm'
$result1 = Join-Path $root 'results\run-1.json'
$result2 = Join-Path $root 'results\run-2.json'

Push-Location (Join-Path $root 'guest')
try {
    cargo build --locked --release --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'sync Component guest build failed' }
} finally {
    Pop-Location
}

Push-Location (Join-Path $root 'host')
try {
    cargo run --locked --release --bin sico-component-host -- $core $component $result1 35
    if ($LASTEXITCODE -ne 0) { throw 'first sync Wasmtime host run failed' }
    cargo run --locked --release --bin sico-component-host -- $core $component $result2 35
    if ($LASTEXITCODE -ne 0) { throw 'second sync Wasmtime host run failed' }
} finally {
    Pop-Location
}

$actual1 = Get-Content -Raw -LiteralPath $result1
$actual2 = Get-Content -Raw -LiteralPath $result2
if ($actual1 -cne $actual2) {
    throw 'determinism check failed: run JSON files differ'
}

$result = $actual1 | ConvertFrom-Json
if ($result.output -ne 50) {
    throw "unexpected output: $($result.output)"
}
if ($result.bigint_roundtrip_bytes -ne 512 -or -not $result.decimal_roundtrip) {
    throw "numeric roundtrip failed: bigint-bytes=$($result.bigint_roundtrip_bytes) decimal=$($result.decimal_roundtrip)"
}
if (-not $result.result_ok_roundtrip -or -not $result.result_error_roundtrip) {
    throw "Result roundtrip failed: ok=$($result.result_ok_roundtrip) error=$($result.result_error_roundtrip)"
}
$expectedEvents = @(
    'counter.new:35',
    'counter.add:7->42',
    'counter.value:42',
    'log:guest subtotal=42',
    'counter.drop:42',
    'host-add:42+8=50'
)
if ((Compare-Object -ReferenceObject $expectedEvents -DifferenceObject @($result.events) -SyncWindow 0)) {
    throw "unexpected event sequence: $($result.events -join ', ')"
}

$componentHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $component).Hash.ToLowerInvariant()
if ($componentHash -cne $result.component_sha256) {
    throw "component hash mismatch: file=$componentHash result=$($result.component_sha256)"
}

Push-Location (Join-Path $root 'async-guest')
try {
    cargo build --locked --release --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'async Component guest build failed' }
} finally {
    Pop-Location
}

Push-Location (Join-Path $root 'host')
try {
    cargo run --locked --release --bin sico-component-async-host -- $asyncCore $asyncComponent 35
    if ($LASTEXITCODE -ne 0) { throw 'first async Wasmtime host run failed' }
    $asyncHash1 = (Get-FileHash -Algorithm SHA256 -LiteralPath $asyncComponent).Hash.ToLowerInvariant()
    cargo run --locked --release --bin sico-component-async-host -- $asyncCore $asyncComponent 35
    if ($LASTEXITCODE -ne 0) { throw 'second async Wasmtime host run failed' }
    $asyncHash2 = (Get-FileHash -Algorithm SHA256 -LiteralPath $asyncComponent).Hash.ToLowerInvariant()
} finally {
    Pop-Location
}
if ($asyncHash1 -cne $asyncHash2) {
    throw "async component determinism check failed: first=$asyncHash1 second=$asyncHash2"
}

Write-Host "PASS component-host-call output=$($result.output) sha256=$componentHash"
Write-Host "PASS component-async output=42 host-calls=1 sha256=$asyncHash1"
