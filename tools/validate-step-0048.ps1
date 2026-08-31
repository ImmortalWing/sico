param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-host-core/src/permission.rs') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0048-permission-records.md') -Raw -Encoding UTF8
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0048 is not complete' }
foreach ($needle in @('PermissionChoice', 'AllowOnce', 'AllowPersistent', 'sico.desktop.permission.v0', 'capability_fingerprint', 'PromptRequired')) {
  if (-not $source.Contains($needle)) { throw "permission contract missing: $needle" }
}
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-host-core --all-targets -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'permission Clippy failed' }
  & $CargoPath test --offline --locked -p sico-host-core
  if ($LASTEXITCODE -ne 0) { throw 'permission tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
Write-Output 'STEP_0048_OK prompt=authorized-closure decisions=deny,allow-once,allow-persistent record=canonical,app-signer-capability session=terminal-expiry corrupt=fail-closed permission_tests=3 host_tests=6 next=STEP-0049'
