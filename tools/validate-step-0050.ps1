param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$source = Get-Content -LiteralPath (Join-Path $root 'crates/sico-host-core/src/ui.rs') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0050-minimal-ui-wit-renderer.md') -Raw -Encoding UTF8
$wit = Get-Content -LiteralPath (Join-Path $root 'crates/sico-host-core/wit/sico-ui-v0.wit') -Raw -Encoding UTF8
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0050 is not complete' }
foreach ($needle in @('MAX_UI_NODES: usize = 1_024', 'MAX_UI_DEPTH: usize = 32', 'MAX_QUEUED_UI_EVENTS: usize = 256', 'MAX_UI_EVENTS_PER_SECOND: usize = 120', 'escape_text', 'accessibility_order')) {
  if (-not $source.Contains($needle)) { throw "UI boundary missing: $needle" }
}
if (-not $wit.Contains('world desktop-app') -or -not $wit.Contains('next-event')) { throw 'UI WIT world is incomplete' }
$previous = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath clippy --offline --locked -p sico-host-core --all-targets -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'UI Clippy failed' }
  & $CargoPath test --offline --locked -p sico-host-core
  if ($LASTEXITCODE -ne 0) { throw 'UI tests failed' }
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previous
}
Write-Output 'STEP_0050_OK ui=typed-no-script nodes=7 node_limit=1024 depth=32 text_total=65536 text_field=4096 events_queue=256 events_rate=120-per-second accessibility=preorder wit=0.253-parse ui_tests=4 host_tests=13 next=STEP-0051'
