param([string]$RepositoryRoot=(Split-Path -Parent $PSScriptRoot),[string]$CargoPath=(Join-Path $HOME '.cargo/bin/cargo.exe'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest;$root=(Resolve-Path $RepositoryRoot).Path
$test=Get-Content -LiteralPath (Join-Path $root 'crates/sico-mobile-host-core/tests/desktop_android_parity.rs') -Raw -Encoding UTF8
$step=Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0060-desktop-android-parity-app.md') -Raw -Encoding UTF8
foreach($needle in @('signed_answer','run_authorized_package','MobileHostCore','revision_digest','desktop_opened')){if(-not $test.Contains($needle)){throw "parity test missing: $needle"}}
if($step -notmatch '(?m)^> - status: partial-runtime-evidence\r?$'){throw 'STEP-0060 must remain partial without Android runtime'}
$previousRuntime=$env:SICO_TEST_WASMTIME;$previousToolchain=$env:RUSTUP_TOOLCHAIN;$env:RUSTUP_TOOLCHAIN='1.97.1-x86_64-pc-windows-gnu';Push-Location $root
try{$env:SICO_TEST_WASMTIME=& (Join-Path $root 'tools/ensure-wasmtime.ps1');& $CargoPath clippy --locked -p sico-mobile-host-core --all-targets -- -D warnings;if($LASTEXITCODE-ne 0){throw 'parity Clippy failed'};& $CargoPath test --locked -p sico-mobile-host-core --test desktop_android_parity;if($LASTEXITCODE-ne 0){throw 'parity tests failed'};foreach($target in 'aarch64-linux-android','x86_64-linux-android'){& $CargoPath check --locked -p sico-mobile-host-core --target $target;if($LASTEXITCODE-ne 0){throw "Android parity core check failed: $target"}}}finally{Pop-Location;$env:SICO_TEST_WASMTIME=$previousRuntime;$env:RUSTUP_TOOLCHAIN=$previousToolchain}
Write-Output 'STEP_0060_PARTIAL package=same-bytes desktop_result=42 metadata=identical mutation=refused android_runtime=unavailable blocker=licensed-sdk-ndk-runner next=STEP-0061'
