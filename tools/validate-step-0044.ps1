param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0019-package-cli-cache-stdio-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0044-package-cli-source-cache.md') -Raw -Encoding UTF8
$languageSource = Get-Content -LiteralPath (Join-Path $root 'crates/sico-cli/src/lib.rs') -Raw -Encoding UTF8
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-app-cli/src/lib.rs') -Raw -Encoding UTF8
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or $step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0044/RFC-0019 is not accepted complete' }
foreach ($needle in @('Command::new("pack")', 'Command::new("inspect")', 'Command::new("run")', 'requires --trusted-key', 'EXIT_TIMEOUT', 'run_authorized_package')) {
  if (-not $source.Contains($needle)) { throw "package CLI/cache contract missing: $needle" }
}
foreach ($removed in @('sico_package', 'sico_runtime', 'Command::new("run")', 'Command::new("inspect")')) {
  if ($languageSource.Contains($removed)) { throw "language CLI still owns application behavior: $removed" }
}
if ($source.Contains('SICO-SOURCE-CACHE-V0') -or $source.Contains('compile_component')) { throw 'application CLI must not compile or cache source' }
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$previousRuntime = $env:SICO_TEST_WASMTIME
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
Push-Location $root
try {
  $env:SICO_TEST_WASMTIME = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
  & $CargoPath clippy --offline --locked -p sico-cli -p sico-app-cli --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'package CLI strict Clippy failed' }
  & $CargoPath test --offline --locked -p sico-cli -p sico-app-cli -- --nocapture
  if ($LASTEXITCODE -ne 0) { throw 'package CLI/cache integration tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
  $env:SICO_TEST_WASMTIME = $previousRuntime
}
Write-Output 'STEP_0044_OK cli=sico-build,sico-app-pack,run,inspect artifact=sapp-v0 inspect=text,json trust=explicit-dev source_cache=removed-by-ADR-0007 args=scalar-refused stdio=guest-separated exits=0-6 next=STEP-0045'
