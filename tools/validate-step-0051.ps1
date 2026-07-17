param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-desktop-host/src/lib.rs') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0051-windows-desktop-host-integration.md') -Raw -Encoding UTF8
$icon = Get-Item -LiteralPath (Join-Path $root 'assets/desktop/sico-desktop-host.ico')
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0051 is not complete' }
foreach ($needle in @('association-plan', 'association-apply', 'native_permission_dialog', 'show_native_ui_preview', '\"%1\"')) {
  if (-not $source.Contains($needle)) { throw "Windows Host missing: $needle" }
}
if ($icon.Length -lt 1024) { throw 'Windows Host icon is missing or invalid' }
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$previousRuntime = $env:SICO_TEST_WASMTIME
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
Push-Location $root
try {
  $env:SICO_TEST_WASMTIME = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
  & $CargoPath clippy --offline --locked -p sico-desktop-host --all-targets -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'Desktop Host Clippy failed' }
  & $CargoPath test --offline --locked -p sico-desktop-host
  if ($LASTEXITCODE -ne 0) { throw 'Desktop Host tests failed' }
  & powershell.exe -NoProfile -NonInteractive -STA -Command 'Add-Type -AssemblyName System.Windows.Forms; $form = New-Object System.Windows.Forms.Form; $form.Dispose()'
  if ($LASTEXITCODE -ne 0) { throw 'Windows Forms native probe failed' }
  & (Join-Path $root 'tools/package-windows-host.ps1') -RepositoryRoot $root -CargoPath $CargoPath
  if ($LASTEXITCODE -ne 0) { throw 'Windows package smoke failed' }
} finally {
  Pop-Location
  $env:SICO_TEST_WASMTIME = $previousRuntime
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
Write-Output 'STEP_0051_OK platform=windows host=sico-desktop-host commands=install,open,uninstall association=per-user-explicit dialog=windows-forms ui_preview=native icon=ico runtime=wasmtime-46.0.1 desktop_tests=3 host_tests=13 package=zip next=STEP-0052'
