param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$android = Read-RepoFile 'docs/platforms/ANDROID-DEVELOPMENT.md'
$harmony = Read-RepoFile 'docs/platforms/HARMONY-DEVELOPMENT.md'
$shared = Read-RepoFile 'docs/platforms/MOBILE-SHARED-CHECKLIST.md'
$index = Read-RepoFile 'docs/platforms/README.md'
$step = Read-RepoFile 'docs/steps/STEP-0070-mobile-platform-development-handbooks.md'
$contract = Read-RepoFile 'tests/platform/mobile-documentation-contract.json' | ConvertFrom-Json
$environment = Read-RepoFile 'tests/platform/environment-2026-07-16-recheck.json' | ConvertFrom-Json

if ($contract.schema -ne 'sico.mobile-documentation-contract.v0' -or $contract.claim -ne 'documentation-only') {
    throw 'mobile documentation contract schema or claim is invalid'
}
if ($contract.common.bridgeEnvelopeBytes -ne 65536 -or $contract.common.packageBytes -ne 67108864 -or $contract.common.inputEventBytes -ne 4096 -or -not $contract.common.copyBeforeVerify) {
    throw 'shared mobile limits or copy-before-verify invariant drifted'
}
if ($contract.android.status -ne 'blocked-not-implemented' -or $contract.android.minSdk -ne 28 -or $contract.android.compileSdk -ne 36 -or $contract.android.targetSdk -ne 36 -or $contract.android.ndk -ne '27.3.13750724') {
    throw 'Android documented status or frozen baseline drifted'
}
if ((@($contract.android.abis) -join ',') -ne 'arm64-v8a,x86_64' -or (@($contract.android.rustTargets) -join ',') -ne 'aarch64-linux-android,x86_64-linux-android') {
    throw 'Android ABI or Rust target matrix drifted'
}
if ($contract.harmony.status -ne 'proposed-not-implemented' -or -not $contract.harmony.variantMustBeSelected -or $contract.harmony.preferredModel -ne 'Stage') {
    throw 'Harmony documented status or variant gate drifted'
}
if ($contract.harmony.bridge -ne 'ArkTS-NodeAPI-C++-stable-C-ABI-Rust' -or (@($contract.harmony.rustTargets) -join ',') -ne 'aarch64-unknown-linux-ohos,x86_64-unknown-linux-ohos') {
    throw 'Harmony bridge or Rust target matrix drifted'
}

foreach ($needle in 'blocked-not-implemented','27.3.13750724','aarch64-linux-android','x86_64-linux-android','copy to private staging','catch_unwind','arm64 physical device','HostActivity.kt','ACTION_OPEN_DOCUMENT','TalkBack','Runtime backend','UnsatisfiedLinkError') {
    if (-not $android.Contains($needle)) { throw "Android handbook missing invariant: $needle" }
}

foreach ($needle in 'proposed / not-implemented','HarmonyOS','OpenHarmony','aarch64-unknown-linux-ohos','x86_64-unknown-linux-ohos','Node-API C++ shim','stable C ABI','UIAbility','onNewWant','HAP/APP','arkXtest','Wasmtime') {
    if (-not $harmony.Contains($needle)) { throw "Harmony handbook missing invariant: $needle" }
}

foreach ($needle in 'copy-before-verify','64 KiB','64 MiB','arm64','Runtime','GO/NO-GO','environment.json','artifacts.sha256') {
    if (-not $shared.Contains($needle)) { throw "shared checklist missing invariant: $needle" }
}
if (-not $index.Contains('ANDROID-DEVELOPMENT.md') -or -not $index.Contains('HARMONY-DEVELOPMENT.md') -or -not $index.Contains('mobile-documentation-contract.json')) {
    throw 'platform documentation index is incomplete'
}
if ($step -notmatch '(?m)^> - status: complete\r?$' -or -not $step.Contains('sequential steps retained their allocated meanings and are now closed locally')) {
    throw 'STEP-0070 record is incomplete or changes the current M7 sequence'
}

$androidLinks = ([regex]::Matches($android, 'https://')).Count
$harmonyLinks = ([regex]::Matches($harmony, 'https://')).Count
if ($androidLinks -lt 15 -or $harmonyLinks -lt 18) { throw 'official reference coverage is unexpectedly low' }

if ($environment.android.sdk -or $environment.android.ndk -or $environment.android.repository_buildable_host -or $environment.harmony.sdk -or $environment.harmony.repository_plan -or $environment.harmony.repository_implementation) {
    throw 'environment recheck no longer matches the documentation-only blocked/proposed claim'
}

Write-Output "STEP_0070_OK android=blocked-not-implemented harmony=proposed-not-implemented android_links=$androidLinks harmony_links=$harmonyLinks sequence=closed-local next=external-evidence-or-STEP-0072"
