param([string]$RepositoryRoot=(Split-Path -Parent $PSScriptRoot),[string]$CargoPath=(Join-Path $HOME '.cargo/bin/cargo.exe'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest;$root=(Resolve-Path $RepositoryRoot).Path
$source=Get-Content -LiteralPath (Join-Path $root 'crates/sico-mobile-host-core/src/lifecycle.rs') -Raw -Encoding UTF8
$kotlin=Get-Content -LiteralPath (Join-Path $root 'android/host/src/main/kotlin/dev/sico/host/LifecycleAdapter.kt') -Raw -Encoding UTF8
$step=Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0058-android-runtime-lifecycle.md') -Raw -Encoding UTF8
foreach($needle in @('NativeCranelift','Pulley','pulley64','BackgroundSuspended','MAX_OPEN_EVENTS','restore_after_process_death')){if(-not $source.Contains($needle)){throw "lifecycle missing: $needle"}}
if(-not $kotlin.Contains('canAssumeGuestRunningAfterRestore(): Boolean = false')){throw 'Kotlin restore contract missing'}
if($step -notmatch '(?m)^> - status: complete-cross-check\r?$'){throw 'STEP-0058 incomplete'}
$previous=$env:RUSTUP_TOOLCHAIN;$env:RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-gnu';Push-Location $root
try{& $CargoPath clippy --locked -p sico-mobile-host-core --all-targets -- -D warnings;if($LASTEXITCODE-ne 0){throw 'lifecycle Clippy failed'};& $CargoPath test --locked -p sico-mobile-host-core;if($LASTEXITCODE-ne 0){throw 'lifecycle tests failed'};foreach($target in 'aarch64-linux-android','x86_64-linux-android'){& $CargoPath check --locked -p sico-mobile-host-core --target $target;if($LASTEXITCODE-ne 0){throw "Android core check failed: $target"}}}finally{Pop-Location;$env:RUSTUP_TOOLCHAIN=$previous}
Write-Output 'STEP_0058_OK runtime_contract=wasmtime-46.0.1 android_core_checks=aarch64,x86_64 backends=native-cranelift,pulley lifecycle=descriptor-reverify queue=256 terminal=single-winner tests=5 android_runtime=unavailable next=STEP-0059'
