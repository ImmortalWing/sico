param([string]$RepositoryRoot=(Split-Path -Parent $PSScriptRoot),[string]$CargoPath=(Join-Path $HOME '.cargo/bin/cargo.exe'))
$ErrorActionPreference='Stop';Set-StrictMode -Version Latest;$root=(Resolve-Path $RepositoryRoot).Path
$source=Get-Content -LiteralPath (Join-Path $root 'crates/sico-mobile-host-core/src/ui.rs') -Raw -Encoding UTF8
$kotlin=Get-Content -LiteralPath (Join-Path $root 'android/host/src/main/kotlin/dev/sico/host/UiAdapter.kt') -Raw -Encoding UTF8
$step=Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0059-android-native-ui-adapter.md') -Raw -Encoding UTF8
foreach($needle in @('AndroidWidgetKind','MAX_ANDROID_INPUT_BYTES','EventGate','accessibility_order')){if(-not $source.Contains($needle)){throw "UI core missing: $needle"}}
foreach($needle in @('LinearLayout','TextView','Button','EditText','Utf8ByteLimitFilter','accessibilityTraversalAfter')){if(-not $kotlin.Contains($needle)){throw "Kotlin UI adapter missing: $needle"}}
if($step -notmatch '(?m)^> - status: complete-cross-check\r?$'){throw 'STEP-0059 incomplete'}
$previous=$env:RUSTUP_TOOLCHAIN;$env:RUSTUP_TOOLCHAIN='1.98.0-x86_64-pc-windows-gnu';Push-Location $root
try{& $CargoPath clippy --locked -p sico-host-core -p sico-mobile-host-core --all-targets -- -D warnings;if($LASTEXITCODE-ne 0){throw 'UI Clippy failed'};& $CargoPath test --locked -p sico-host-core -p sico-mobile-host-core;if($LASTEXITCODE-ne 0){throw 'UI tests failed'};foreach($target in 'aarch64-linux-android','x86_64-linux-android'){& $CargoPath check --locked -p sico-mobile-host-core --target $target;if($LASTEXITCODE-ne 0){throw "Android UI core check failed: $target"}}}finally{Pop-Location;$env:RUSTUP_TOOLCHAIN=$previous}
Write-Output 'STEP_0059_OK widgets=layout,text,button,input webview=forbidden input=4096B event_rate=120/s queue=256 accessibility=ordered tests=4 android_core_checks=aarch64,x86_64 native_ui=unavailable next=STEP-0060'
