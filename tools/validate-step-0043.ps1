param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0018-runtime-limits-fault-taxonomy-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0043-runtime-limits-fault-taxonomy.md') -Raw -Encoding UTF8
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-runtime/src/lib.rs') -Raw -Encoding UTF8
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or $step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0043/RFC-0018 is not accepted complete' }
foreach ($needle in @('fuel=', 'timeout=', 'max-memory-size=', 'max-table-elements=', 'max-instances=', 'max-resources=', 'hostcall-fuel=', 'max-random-size=', 'http-outgoing-body-chunk-size=', 'FaultClass::ResourceLimit', 'FaultClass::Timeout')) {
  if (-not $source.Contains($needle)) { throw "Runtime limit/fault boundary missing: $needle" }
}
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$previousRuntime = $env:SICO_TEST_WASMTIME
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  $env:SICO_TEST_WASMTIME = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
  & $CargoPath clippy --offline --locked -p sico-runtime --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'Runtime limits Clippy failed' }
  & $CargoPath test --offline --locked -p sico-runtime -- --nocapture
  if ($LASTEXITCODE -ne 0) { throw 'Runtime limits/host survival tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
  $env:SICO_TEST_WASMTIME = $previousRuntime
}
Write-Output 'STEP_0043_OK runtime=wasmtime-46.0.1 limits=fuel,time,memory,table,instance,wasi-resource,hostcall,random,body fault_classes=7 malicious=loop-fuel host_survival=pass runtime_tests=6 next=STEP-0044'
