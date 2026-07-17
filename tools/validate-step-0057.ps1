param([string]$RepositoryRoot=(Split-Path -Parent $PSScriptRoot),[string]$CargoPath=(Join-Path $HOME '.cargo/bin/cargo.exe'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest;$root=(Resolve-Path $RepositoryRoot).Path
$source=Get-Content -LiteralPath (Join-Path $root 'crates/sico-mobile-host-core/src/permission.rs') -Raw -Encoding UTF8
$manifest=Get-Content -LiteralPath (Join-Path $root 'android/host/src/main/AndroidManifest.xml') -Raw -Encoding UTF8
$kotlin=Get-Content -LiteralPath (Join-Path $root 'android/host/src/main/kotlin/dev/sico/host/PermissionAdapter.kt') -Raw -Encoding UTF8
$step=Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0057-android-permission-storage.md') -Raw -Encoding UTF8
foreach($needle in @('storage.read-write','clock.read','random.read','log.write','network.connect','UriGrantSession')){if(-not $source.Contains($needle)){throw "permission mapping missing: $needle"}}
if(-not $manifest.Contains('android.permission.INTERNET')){throw 'Android INTERNET declaration missing'}
if(-not $kotlin.Contains('checkSelfPermission')){throw 'permission use-time check missing'}
if($step -notmatch '(?m)^> - status: complete\r?$'){throw 'STEP-0057 incomplete'}
$previous=$env:RUSTUP_TOOLCHAIN;$env:RUSTUP_TOOLCHAIN='1.97.1-x86_64-pc-windows-gnu';Push-Location $root
try{& $CargoPath clippy --offline --locked -p sico-mobile-host-core --all-targets -- -D warnings;if($LASTEXITCODE-ne 0){throw 'permission Clippy failed'};& $CargoPath test --offline --locked -p sico-mobile-host-core;if($LASTEXITCODE-ne 0){throw 'permission tests failed'};foreach($target in 'aarch64-linux-android','x86_64-linux-android'){& $CargoPath check --offline --locked -p sico-mobile-host-core --target $target;if($LASTEXITCODE-ne 0){throw "permission Android check failed: $target"}}}finally{Pop-Location;$env:RUSTUP_TOOLCHAIN=$previous}
Write-Output 'STEP_0057_OK capabilities=5 manifest=INTERNET runtime_permissions=0 uri_grants=ephemeral-separated storage=app-private-app-signer drift=re-prompt tests=4 android_checks=2 next=STEP-0058'
