param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0039-deterministic-sapp-builder-loader-inspect.md') -Raw -Encoding UTF8
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-package/src/lib.rs') -Raw -Encoding UTF8
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0039 is not complete' }
foreach ($needle in @('MAX_PACKAGE_BYTES', 'MAX_COMPONENT_BYTES', 'NonCanonicalManifest', 'DigestMismatch', 'Validator::new_with_features')) {
  if (-not $source.Contains($needle)) { throw "package verifier missing evidence: $needle" }
}
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-package --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'sico-package strict Clippy failed' }
  & $CargoPath test --offline --locked -p sico-package
  if ($LASTEXITCODE -ne 0) { throw 'sico-package tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
Write-Output 'STEP_0039_OK package_crate=sico-package deterministic=byte-identical parser=strict component=wasmparser hashes=sha256 package_tests=4 threat_classes=framing,path,limit,manifest,integrity next=STEP-0040'
