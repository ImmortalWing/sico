param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe'),
  [int]$IterationsPerRun = 20
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$work = Join-Path $root 'target/m5/performance'
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
Push-Location $root
try {
  & $CargoPath build --offline --locked --release -p sico-cli -p sico-desktop-host
  if ($LASTEXITCODE -ne 0) { throw 'M5 release binaries failed to build' }
  if (Test-Path -LiteralPath $work) {
    $resolved = (Resolve-Path -LiteralPath $work).Path
    if (-not $resolved.StartsWith((Join-Path $root 'target'))) { throw 'unsafe performance path' }
    Remove-Item -LiteralPath $resolved -Recurse -Force
  }
  New-Item -ItemType Directory -Path $work | Out-Null
  $seed = Join-Path $work 'development-seed.hex'
  $package = Join-Path $work 'hello-desktop.sapp'
  $key = Join-Path $work 'trusted-key.hex'
  $store = Join-Path $work 'store'
  [IO.File]::WriteAllText($seed, "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f`n")
  $sico = Join-Path $root 'target/release/sico.exe'
  $hostExe = Join-Path $root 'target/release/sico-desktop-host.exe'
  $runtime = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
  & $sico build --sign-key $seed --output $package (Join-Path $root 'examples/desktop/hello-desktop.sico') | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'representative package build failed' }
  $inspected = (& $sico inspect --json $package | ConvertFrom-Json)
  [IO.File]::WriteAllText($key, "$($inspected.trust.public_key)`n")
  & $hostExe open $package --store $store --trusted-key $key --runtime $runtime | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'representative host warmup failed' }
  $runs = @()
  foreach ($run in 1..3) {
    $timer = [Diagnostics.Stopwatch]::StartNew()
    foreach ($iteration in 1..$IterationsPerRun) {
      & $hostExe open $package --store $store --trusted-key $key --runtime $runtime | Out-Null
      if ($LASTEXITCODE -ne 0) { throw "representative host launch failed at run $run iteration $iteration" }
    }
    $timer.Stop()
    $runs += [ordered]@{
      run = $run
      total_ms = [Math]::Round($timer.Elapsed.TotalMilliseconds, 3)
      mean_startup_ms = [Math]::Round($timer.Elapsed.TotalMilliseconds / $IterationsPerRun, 3)
    }
  }
  $sorted = @($runs.mean_startup_ms | Sort-Object)
  $record = [ordered]@{
    schema = 'sico.m5.desktop-host-startup-runs.v0'
    date = '2026-07-16'
    environment = [ordered]@{ os = 'Windows'; rust = '1.97.0'; host = 'x86_64-pc-windows-gnu'; profile = 'release'; runtime = 'wasmtime-46.0.1' }
    workload = 'signed package reinstall/reverify/open plus Wasmtime process execution returning 42'
    iterations_per_run = $IterationsPerRun
    sla = 'not-established'
    runs = $runs
    median_mean_startup_ms = $sorted[1]
  }
  $json = $record | ConvertTo-Json -Depth 6
  $output = Join-Path $work 'm5-desktop-host-windows-release.json'
  [IO.File]::WriteAllText($output, "$json`n")
  Write-Output "M5_PERFORMANCE_OK runs=3 iterations=$IterationsPerRun median_mean_startup_ms=$($sorted[1]) output=$output"
} finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
