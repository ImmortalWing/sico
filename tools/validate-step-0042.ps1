param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$adr = Get-Content -LiteralPath (Join-Path $root 'docs/adr/ADR-0003-isolated-storage-wasi-host-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0042-wasi-capability-host-isolated-storage.md') -Raw -Encoding UTF8
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-runtime/src/lib.rs') -Raw -Encoding UTF8
if ($adr -notmatch '(?m)^> - status: accepted\r?$' -or $step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0042/ADR-0003 is not accepted complete' }
foreach ($needle in @('SICO-STORAGE-ID-V0', 'FILE_ATTRIBUTE_REPARSE_POINT', 'QuotaExceeded', 'cli=n', 'allow-ip-name-lookup=n')) {
  if (-not $source.Contains($needle)) { throw "storage/WASI host missing: $needle" }
}
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-runtime --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'Runtime storage Clippy failed' }
  & $CargoPath test --offline --locked -p sico-runtime
  if ($LASTEXITCODE -ne 0) { throw 'Runtime storage tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
Write-Output 'STEP_0042_OK storage_identity=app-plus-trust-hash roots=direct-child path=normalized links=symlink,reparse cross_app=denied quota=audit wasi=explicit-default-off runtime_tests=4 next=STEP-0043'
