param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-desktop-host/src/platform.rs') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0052-desktop-platform-adapters-parity.md') -Raw -Encoding UTF8
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0052 is not complete' }
foreach ($needle in @('RuntimeVerified', 'ContractVerified', 'CFBundleDocumentTypes', 'LSHandlerRank', 'Exec=sico-desktop-host open %f', 'application/vnd.sico.sapp')) {
  if (-not $source.Contains($needle)) { throw "Platform adapter missing: $needle" }
}
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$previousRuntime = $env:SICO_TEST_WASMTIME
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$output = Join-Path $root 'target/m5/step0052-contracts'
Push-Location $root
try {
  $env:SICO_TEST_WASMTIME = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
  & $CargoPath clippy --offline --locked -p sico-desktop-host --all-targets -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'Platform adapter Clippy failed' }
  & $CargoPath test --offline --locked -p sico-desktop-host
  if ($LASTEXITCODE -ne 0) { throw 'Platform adapter tests failed' }
  & $CargoPath build --offline --locked --release -p sico-desktop-host
  if ($LASTEXITCODE -ne 0) { throw 'Windows Host release build failed' }
  if (Test-Path -LiteralPath $output) {
    $resolved = (Resolve-Path -LiteralPath $output).Path
    if (-not $resolved.StartsWith((Join-Path $root 'target'))) { throw 'unsafe platform output path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
  & (Join-Path $root 'target/release/sico-desktop-host.exe') platform-artifacts --output $output --executable (Join-Path $root 'target/release/sico-desktop-host.exe')
  if ($LASTEXITCODE -ne 0) { throw 'Platform artifact generation failed' }
  foreach ($relative in @('contracts.json', 'windows-association-plan.json', 'macos/Info.plist', 'linux/sico.desktop', 'linux/sico-sapp.xml', 'linux/mimeapps.list')) {
    if (-not (Test-Path -LiteralPath (Join-Path $output $relative) -PathType Leaf)) { throw "Platform artifact missing: $relative" }
  }
} finally {
  Pop-Location
  $env:SICO_TEST_WASMTIME = $previousRuntime
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
Write-Output 'STEP_0052_OK adapters=windows,macos,linux parity=extension,identity,open-argument windows=runtime-verified macos=contract-verified linux=contract-verified unavailable_runners=macos,linux platform_tests=3 next=STEP-0053'

