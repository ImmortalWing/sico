param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe'),
    [string]$NodePath = 'C:/Program Files/nodejs/node.exe'
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0011-deterministic-core-wasm-backend-v0.md') -Raw -Encoding UTF8
$snapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/wasm/artifacts.hex') -Encoding UTF8 | Where-Object { $_ -match '^(numeric|control)=' })

if ($rfc -notmatch '(?m)^> - status: accepted\r?$') { throw 'Core Wasm backend contract is not accepted' }
if ($snapshots.Count -ne 2) { throw "expected 2 Core Wasm artifacts, found $($snapshots.Count)" }
if (-not (Test-Path -LiteralPath $NodePath -PathType Leaf)) { throw "Node WebAssembly engine not found: $NodePath" }

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = 'stable'
Push-Location $root
try {
    & $CargoPath test --offline --locked -p sico-codegen-wasm
    if ($LASTEXITCODE -ne 0) {
        throw "Core Wasm backend tests failed"
    }
    & $CargoPath clippy --offline --locked -p sico-codegen-wasm --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) {
        throw "Core Wasm strict Clippy failed"
    }
    & $NodePath tools/validate-core-wasm-runtime.mjs
    if ($LASTEXITCODE -ne 0) {
        throw "Core Wasm engine execution failed"
    }

    Write-Output "STEP_0033_OK artifacts=2 deterministic=byte-identical validator=wasmparser-0.253 engine=node-webassembly numeric=42 control_true=7 control_false=9 invalid_ir=blocked arbitrary_int=refused wasm_abi=bounded-scalar-probe"
}
finally {
    Pop-Location
    $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
