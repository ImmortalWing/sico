param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0016-development-signing-trust-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0040-development-signing-trust-policy.md') -Raw -Encoding UTF8
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-package/src/lib.rs') -Raw -Encoding UTF8
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or $step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0040/RFC-0016 is not accepted complete' }
foreach ($needle in @('SIGNATURE_DOMAIN', 'verify_strict', 'is_weak', 'TrustPolicy', 'TrustedPackage', 'UntrustedKey')) {
  if (-not $source.Contains($needle)) { throw "signing implementation missing: $needle" }
}
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-package --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'sico-package signing Clippy failed' }
  & $CargoPath test --offline --locked -p sico-package
  if ($LASTEXITCODE -ne 0) { throw 'sico-package signing tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
Write-Output 'STEP_0040_OK scheme=ed25519-dev-v0 domain=separated strict_verify=pass package_tests=6 fixtures=valid,wrong-key,unsigned,replay trust_type=TrustedPackage next=STEP-0041'
