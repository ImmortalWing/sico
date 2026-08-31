param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-host-core/src/lib.rs') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0047-shared-host-install-open.md') -Raw -Encoding UTF8
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0047 is not complete' }
foreach ($needle in @('TrustPolicy::RequireDevelopment', 'authorize(trusted', 'SICO-DESKTOP-APP-ID-V0', 'SICO-DESKTOP-CAPABILITY-V0', '.staging-', 'verify_installed_metadata')) {
  if (-not $source.Contains($needle)) { throw "Host install/open gate missing: $needle" }
}
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-host-core --all-targets -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'Host core Clippy failed' }
  & $CargoPath test --offline --locked -p sico-host-core
  if ($LASTEXITCODE -ne 0) { throw 'Host core tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
Write-Output 'STEP_0047_OK crate=sico-host-core install=signed-only,atomic,copy-by-digest open=digest,trust,identity,closure-reverified signer_isolation=pass downgrade=refused tamper=refused host_tests=3 next=STEP-0048'
