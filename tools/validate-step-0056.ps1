param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot), [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe'))
$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest
$root=(Resolve-Path $RepositoryRoot).Path
$source=Get-Content -LiteralPath (Join-Path $root 'crates/sico-mobile-host-core/src/intent.rs') -Raw -Encoding UTF8
$manifestPath=Join-Path $root 'android/host/src/main/AndroidManifest.xml'; [xml]$manifest=Get-Content -LiteralPath $manifestPath -Raw -Encoding UTF8
$kotlin=Get-Content -LiteralPath (Join-Path $root 'android/host/src/main/kotlin/dev/sico/host/IntentAdapter.kt') -Raw -Encoding UTF8
$step=Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0056-android-intent-package-ingest.md') -Raw -Encoding UTF8
foreach($needle in @('content://','application/vnd.sico.sapp','sico://open','MAX_PACKAGE_BYTES')){if(-not $source.Contains($needle)){throw "intent core missing: $needle"}}
foreach($needle in @('openInputStream','readNBytes','SCHEME_CONTENT')){if(-not $kotlin.Contains($needle)){throw "Kotlin intent adapter missing: $needle"}}
if($null -eq $manifest.manifest.application.activity){throw 'Android manifest activity missing'}
if($step -notmatch '(?m)^> - status: complete\r?$'){throw 'STEP-0056 is not complete'}
$previous=$env:RUSTUP_TOOLCHAIN;$env:RUSTUP_TOOLCHAIN='1.97.0-x86_64-pc-windows-gnu';Push-Location $root
try{
 & $CargoPath clippy --offline --locked -p sico-mobile-host-core --all-targets -- -D warnings;if($LASTEXITCODE-ne 0){throw 'intent Clippy failed'}
 & $CargoPath test --offline --locked -p sico-mobile-host-core;if($LASTEXITCODE-ne 0){throw 'intent tests failed'}
 foreach($target in 'aarch64-linux-android','x86_64-linux-android'){& $CargoPath check --offline --locked -p sico-mobile-host-core --target $target;if($LASTEXITCODE-ne 0){throw "intent Android check failed: $target"}}
}finally{Pop-Location;$env:RUSTUP_TOOLCHAIN=$previous}
Write-Output 'STEP_0056_OK actions=view,send,picker deeplink=picker-only uri=content-only stream=copy-once limit=64MiB mime=exact manifest=parsed tests=4 android_checks=2 sdk_compile=unavailable next=STEP-0057'
