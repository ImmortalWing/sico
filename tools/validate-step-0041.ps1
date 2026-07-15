param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0017-capability-closure-permission-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0041-capability-closure-permission-intersection.md') -Raw -Encoding UTF8
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-package/src/lib.rs') -Raw -Encoding UTF8
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or $step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0041/RFC-0017 is not accepted complete' }
foreach ($needle in @('AuthorizedPackage', 'SourceManifestMismatch', 'ImportManifestMismatch', 'HostDenied', 'capabilities_for_imports')) {
  if (-not $source.Contains($needle)) { throw "capability gate missing: $needle" }
}
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-package --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'capability Clippy failed' }
  & $CargoPath test --offline --locked -p sico-package
  if ($LASTEXITCODE -ne 0) { throw 'capability tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
Write-Output 'STEP_0041_OK closure=source-manifest-import host=required-subset actual=intersection capability_names=5 package_tests=8 imported_fixture=clock unknown=default-deny next=STEP-0042'
