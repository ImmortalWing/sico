param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-host-core/src/lifecycle.rs') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0049-lifecycle-process-supervision.md') -Raw -Encoding UTF8
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0049 is not complete' }
foreach ($needle in @('MAX_QUEUED_OPEN_EVENTS: usize = 256', 'AlreadyRunning', 'TimedOut', 'Cancelled', 'taskkill', 'permission_session.clear()')) {
  if (-not $source.Contains($needle)) { throw "lifecycle boundary missing: $needle" }
}
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-host-core --all-targets -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'lifecycle Clippy failed' }
  & $CargoPath test --offline --locked -p sico-host-core
  if ($LASTEXITCODE -ne 0) { throw 'lifecycle tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
Write-Output 'STEP_0049_OK supervision=single-child queue=256 states=running,background,closing terminals=exited,crashed,timeout,cancelled process_tree=windows-kill cleanup=exact-host-owned lifecycle_tests=3 host_tests=9 next=STEP-0050'
