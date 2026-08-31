param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe'),
    [string]$WasmtimePath = '',
    [string]$BinutilsPath = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0012-component-wit-boundary-v0.md') -Raw -Encoding UTF8
$wit = Get-Content -LiteralPath (Join-Path $root 'wit/boundary-probe-v0/world.wit') -Raw -Encoding UTF8
$snapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/wasm/artifacts.hex') -Encoding UTF8 | Where-Object { $_ -match '^(numeric|control)-component=' })

if ($rfc -notmatch '(?m)^> - status: accepted\r?$') { throw 'Component/WIT boundary contract is not accepted' }
foreach ($shape in @('record big-int', 'record decimal-value', 'result<u32, boundary-error>', 'resource counter')) {
    if (-not $wit.Contains($shape)) { throw "WIT boundary probe is missing: $shape" }
}
if ($snapshots.Count -ne 2) { throw "expected 2 Component artifacts, found $($snapshots.Count)" }

if (-not $WasmtimePath) {
    $WasmtimePath = & (Join-Path $root 'tools/ensure-wasmtime.ps1') -RepositoryRoot $root
}
if (-not (Test-Path -LiteralPath $WasmtimePath -PathType Leaf)) { throw "Wasmtime not found: $WasmtimePath" }
if (-not $BinutilsPath) {
    $candidate = Join-Path $root 'target/tooling/msys2-binutils/mingw64/bin'
    if (Test-Path -LiteralPath $candidate -PathType Container) { $BinutilsPath = $candidate }
    if (-not $BinutilsPath) {
        $candidate = Join-Path $HOME '.rustup/toolchains/1.98.0-x86_64-pc-windows-gnu/lib/rustlib/x86_64-pc-windows-gnu/bin/self-contained'
        if (Test-Path -LiteralPath $candidate -PathType Container) { $BinutilsPath = $candidate }
    }
}
$cargoDirectory = Split-Path -Parent $CargoPath
if ($BinutilsPath) { $env:Path = "$BinutilsPath;$cargoDirectory;$env:Path" } else { $env:Path = "$cargoDirectory;$env:Path" }

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$artifact = Join-Path $root 'target/step-0034/numeric.component.wasm'
Push-Location $root
try {
    & $CargoPath test --offline --locked -p sico-codegen-wasm
    if ($LASTEXITCODE -ne 0) { throw 'Component backend tests failed' }
    & $CargoPath clippy --offline --locked -p sico-codegen-wasm --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'Component backend strict Clippy failed' }
    & $CargoPath run --offline --locked -p sico-codegen-wasm --example emit_component -- $artifact
    if ($LASTEXITCODE -ne 0) { throw 'compiler Component emission failed' }
    $runtimeOutput = (& $WasmtimePath run --codegen cache=n --invoke 'main()' $artifact | Out-String).Trim()
    if ($LASTEXITCODE -ne 0 -or $runtimeOutput -cne '42') {
        throw "selected Wasmtime Runtime returned unexpected output: $runtimeOutput"
    }
    & powershell -NoProfile -ExecutionPolicy Bypass -File prototypes/component-host-call/tools/validate.ps1
    if ($LASTEXITCODE -ne 0) { throw 'WIT Result/record/resource host-call roundtrip failed' }

    Write-Output 'STEP_0034_OK compiler_components=2 deterministic=byte-identical validator=wasmparser-0.253 runtime=wasmtime-46.0.1 compiler_result=42 wit_parser=0.253 result_ok=pass result_error=pass records=big-int,decimal resource=owned-borrowed-drop host_output=50'
}
finally {
    Pop-Location
    $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
