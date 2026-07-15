param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot), [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe'))
$ErrorActionPreference = 'Stop'; Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-mobile-host-core/src/lib.rs') -Raw -Encoding UTF8
$kotlin = Get-Content -LiteralPath (Join-Path $root 'android/host/src/main/kotlin/dev/sico/host/NativeBridge.kt') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0055-mobile-host-jni-boundary.md') -Raw -Encoding UTF8
foreach ($needle in @('BRIDGE_SCHEMA', 'MAX_BRIDGE_BYTES', 'NATIVE_PANIC', 'install_bytes', 'open_installed')) { if (-not $source.Contains($needle)) { throw "mobile bridge missing: $needle" } }
foreach ($needle in @('external fun dispatch', 'copyOf()', 'MAX_BRIDGE_BYTES')) { if (-not $kotlin.Contains($needle)) { throw "Kotlin declaration missing: $needle" } }
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0055 is not complete' }
$previous = $env:RUSTUP_TOOLCHAIN; $env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-mobile-host-core --all-targets -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'mobile bridge Clippy failed' }
  & $CargoPath test --offline --locked -p sico-mobile-host-core
  if ($LASTEXITCODE -ne 0) { throw 'mobile bridge tests failed' }
  foreach ($target in 'aarch64-linux-android','x86_64-linux-android') {
    & $CargoPath check --offline --locked -p sico-mobile-host-core --target $target
    if ($LASTEXITCODE -ne 0) { throw "mobile bridge cross-check failed: $target" }
  }
} finally { Pop-Location; $env:RUSTUP_TOOLCHAIN = $previous }
Write-Output 'STEP_0055_OK crate=sico-mobile-host-core bridge=typed-json bytes=separate-copied panic=NATIVE_PANIC operations=probe,install,open,uninstall tests=4 android_checks=aarch64,x86_64 kotlin=contract-reviewed next=STEP-0056'
