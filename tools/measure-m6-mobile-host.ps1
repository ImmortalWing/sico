param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe'),
  [int]$IterationsPerRun = 10000,
  [string]$OutputPath
)
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest;$root=(Resolve-Path $RepositoryRoot).Path
$previous=$env:RUSTUP_TOOLCHAIN;$env:RUSTUP_TOOLCHAIN='1.97.0-x86_64-pc-windows-gnu';Push-Location $root
try {
  & $CargoPath build --offline --locked --release -p sico-mobile-host-core --example mobile_bridge_probe
  if($LASTEXITCODE-ne 0){throw 'mobile bridge release probe build failed'}
  $probe=Join-Path $root 'target/release/examples/mobile_bridge_probe.exe';$runs=@()
  foreach($run in 1..3){$result=(& $probe $IterationsPerRun | ConvertFrom-Json);$runs += [ordered]@{run=$run;iterations=$result.iterations;elapsed_ms=[Math]::Round($result.elapsed_ns/1000000,3);mean_dispatch_us=[Math]::Round($result.elapsed_ns/$result.iterations/1000,3)}}
  $sorted=@($runs.mean_dispatch_us|Sort-Object);$record=[ordered]@{schema='sico.m6.mobile-host-bridge-probe.v0';date='2026-07-16';environment=[ordered]@{os='Windows';rust='1.97.0';host='x86_64-pc-windows-gnu';profile='release'};workload='typed probe dispatch through Android-neutral Mobile Host core';iterations_per_run=$IterationsPerRun;sla='not-established';android_runtime='unavailable';runs=$runs;median_mean_dispatch_us=$sorted[1]}
  $output=if($OutputPath){$OutputPath}else{Join-Path $root 'tests/performance/m6-mobile-host-bridge-windows-release.json'};$parent=Split-Path -Parent $output;if($parent){New-Item -ItemType Directory -Force -Path $parent|Out-Null};[IO.File]::WriteAllText($output,"$($record|ConvertTo-Json -Depth 6)`n")
  Write-Output "M6_PERFORMANCE_PARTIAL runs=3 iterations=$IterationsPerRun median_mean_dispatch_us=$($sorted[1]) android_startup=unavailable output=$output"
} finally {Pop-Location;$env:RUSTUP_TOOLCHAIN=$previous}
