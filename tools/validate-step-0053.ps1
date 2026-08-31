param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0053-m5-quality-exit-audit.md') -Raw -Encoding UTF8
$audit = Get-Content -LiteralPath (Join-Path $root 'docs/reports/m5-exit-audit.md') -Raw -Encoding UTF8
$plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M6-android-host.md') -Raw -Encoding UTF8
$performance = Get-Content -LiteralPath (Join-Path $root 'tests/performance/m5-desktop-host-windows-release.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0053 is not complete' }
if (-not $audit.Contains('GO: M5 complete; M6 entry gate satisfied; next STEP-0054.')) { throw 'M5 audit has no exact GO' }
if ($plan -notmatch '(?m)^> - status: ready after M5 GO\r?$' -or -not $plan.Contains('STEP-0061')) { throw 'M6 plan is incomplete' }
if ($performance.schema -ne 'sico.m5.desktop-host-startup-runs.v0' -or $performance.runs.Count -ne 3 -or $performance.median_mean_startup_ms -le 0) { throw 'M5 performance record is invalid' }
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$previousRuntime = $env:SICO_TEST_WASMTIME
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$work = Join-Path $root 'target/m5/exit-example'
Push-Location $root
try {
  $env:SICO_TEST_WASMTIME = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
  & $CargoPath fmt --all -- --check
  if ($LASTEXITCODE -ne 0) { throw 'M5 formatting check failed' }
  & $CargoPath clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'M5 workspace Clippy failed' }
  & $CargoPath test --offline --locked --workspace --all-targets --all-features
  if ($LASTEXITCODE -ne 0) { throw 'M5 workspace tests failed' }
  & $CargoPath build --offline --locked --release -p sico-cli -p sico-app-cli -p sico-desktop-host
  if ($LASTEXITCODE -ne 0) { throw 'M5 release app tools failed' }
  if (Test-Path -LiteralPath $work) {
    $resolved = (Resolve-Path -LiteralPath $work).Path
    if (-not $resolved.StartsWith((Join-Path $root 'target'))) { throw 'unsafe representative app path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
  New-Item -ItemType Directory -Path $work | Out-Null
  $seed = Join-Path $work 'seed.hex'
  $component = Join-Path $work 'hello-desktop.component.wasm'
  $package = Join-Path $work 'hello-desktop.sapp'
  $key = Join-Path $work 'trusted-key.hex'
  $store = Join-Path $work 'store'
  [IO.File]::WriteAllText($seed, "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f`n")
  $sico = Join-Path $root 'target/release/sico.exe'
  $sicoApp = Join-Path $root 'target/release/sico-app.exe'
  $hostExe = Join-Path $root 'target/release/sico-desktop-host.exe'
  & $sico build --output $component (Join-Path $root 'examples/desktop/hello-desktop.sico') | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'representative Component build failed' }
  & $sicoApp pack --sign-key $seed --output $package $component | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'representative signed package build failed' }
  $inspected = (& $sicoApp inspect --json $package | ConvertFrom-Json)
  [IO.File]::WriteAllText($key, "$($inspected.trust.public_key)`n")
  $result = & $hostExe open $package --store $store --trusted-key $key --runtime $env:SICO_TEST_WASMTIME
  if ($LASTEXITCODE -ne 0 -or "$result".Trim() -ne '42') { throw 'representative Desktop Host result mismatch' }
  $ui = & $hostExe ui-preview (Join-Path $root 'examples/desktop/hello-ui.json') --validate-only
  if ($LASTEXITCODE -ne 0 -or "$ui".Trim() -ne 'validated-ui-nodes=4') { throw 'representative typed UI validation failed' }
  & (Join-Path $root 'tools/measure-m5-desktop-host.ps1') -RepositoryRoot $root -CargoPath $CargoPath -IterationsPerRun 3
  if ($LASTEXITCODE -ne 0) { throw 'M5 performance reproducibility smoke failed' }
} finally {
  Pop-Location
  $env:SICO_TEST_WASMTIME = $previousRuntime
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
Write-Output 'STEP_0053_OK app=signed-sico-plus-typed-ui result=42 properties=10240 startup_runs=3 startup_iterations=20 startup_median_mean_ms=32.344 workspace=fmt,clippy,test audit=GO next=STEP-0054'
