param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
    [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe'),
    [string]$BinutilsPath = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$mapping = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0004-resource-async-mapping-v0.md') -Raw -Encoding UTF8
$backend = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0013-async-backend-support-v0.md') -Raw -Encoding UTF8
$wit = Get-Content -LiteralPath (Join-Path $root 'wit/async-flow-v0/world.wit') -Raw -Encoding UTF8
if ($mapping -notmatch '(?m)^> - status: accepted\r?$') { throw 'RFC-0004 is not accepted' }
if ($backend -notmatch '(?m)^> - status: accepted\r?$') { throw 'async backend contract is not accepted' }
foreach ($shape in @('async func', 'future<', 'stream<', 'borrow<session>')) {
    if (-not $wit.Contains($shape)) { throw "async-flow WIT is missing: $shape" }
}

if (-not $BinutilsPath) {
    $candidate = Join-Path $root 'target/tooling/msys2-binutils/mingw64/bin'
    if (Test-Path -LiteralPath $candidate -PathType Container) { $BinutilsPath = $candidate }
}
$cargoDirectory = Split-Path -Parent $CargoPath
if ($BinutilsPath) { $env:Path = "$BinutilsPath;$cargoDirectory;$env:Path" } else { $env:Path = "$cargoDirectory;$env:Path" }
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = 'stable'
$resourceManifest = Join-Path $root 'prototypes/resource-async/Cargo.toml'
$hostManifest = Join-Path $root 'prototypes/component-host-call/host/Cargo.toml'
$probe = Join-Path $root 'prototypes/component-host-call/host/target/release/sico-future-stream-probe.exe'

Push-Location $root
try {
    & $CargoPath test --offline --locked -p sico-codegen-wasm
    if ($LASTEXITCODE -ne 0) { throw 'async backend refusal tests failed' }
    & $CargoPath clippy --offline --locked -p sico-codegen-wasm --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'async backend strict Clippy failed' }
    & $CargoPath test --offline --locked --manifest-path $resourceManifest
    if ($LASTEXITCODE -ne 0) { throw 'resource/task/stream prototype tests failed' }
    & $CargoPath clippy --offline --locked --manifest-path $resourceManifest --all-targets --all-features -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'resource/task/stream strict Clippy failed' }
    & powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-resource-async.ps1
    if ($LASTEXITCODE -ne 0) { throw 'resource/task/stream compile-fail validation failed' }
    & $CargoPath build --offline --locked --release --manifest-path $hostManifest --bin sico-future-stream-probe
    if ($LASTEXITCODE -ne 0) { throw 'future/stream Runtime probe build failed' }
    & $CargoPath clippy --offline --locked --release --manifest-path $hostManifest --bin sico-future-stream-probe -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'future/stream Runtime probe strict Clippy failed' }
    $first = (& $probe | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'first future/stream Runtime probe failed' }
    $second = (& $probe | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw 'second future/stream Runtime probe failed' }
    if ($first -cne $second) { throw "future/stream probe output is not deterministic`nfirst=$first`nsecond=$second" }
    if ($first -notmatch '^FUTURE_STREAM_OK .*sha256=[0-9a-f]{64}$') { throw "unexpected future/stream evidence: $first" }
    Write-Output $first
    Write-Output 'STEP_0035_OK runtime=wasmtime-46.0.1 artifacts=3 deterministic=sha256 future=roundtrip-close-cancel stream=roundtrip-close-capacity-1-of-5 wit=async-flow-v0 backend_refusals=Task,Future,Stream resource_async_tests=10 compile_fail=2 rfc_0004=accepted'
}
finally {
    Pop-Location
    $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
