param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

$threats = Get-Content -LiteralPath (Join-Path $root 'tests/ecosystem/threat-matrix.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$compat = Get-Content -LiteralPath (Join-Path $root 'tests/ecosystem/compatibility-matrix.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$environment = Get-Content -LiteralPath (Join-Path $root 'tests/platform/environment-2026-07-16-recheck.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$runner = Get-Content -LiteralPath (Join-Path $root 'tests/android-host/runner-gate.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$adr = Get-Content -LiteralPath (Join-Path $root 'docs/adr/ADR-0006-ecosystem-release-trust-boundary-v0.md') -Raw -Encoding UTF8
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0022-ecosystem-compatibility-contract-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0062-ecosystem-release-contract.md') -Raw -Encoding UTF8
$plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M7-ecosystem-release.md') -Raw -Encoding UTF8

if ($threats.schema -ne 'sico.ecosystem.threat-matrix.v0' -or @($threats.cases).Count -ne 32 -or @($threats.cases.id | Sort-Object -Unique).Count -ne 32) {
    throw 'ecosystem threat matrix must contain 32 unique cases'
}
foreach ($area in 'publisher','namespace','registry','update','dependency','disclosure','rollback','compatibility') {
    if (@($threats.cases | Where-Object area -eq $area).Count -ne 4) { throw "ecosystem threat area must contain four cases: $area" }
}
foreach ($case in $threats.cases) {
    if ([string]::IsNullOrWhiteSpace($case.threat) -or [string]::IsNullOrWhiteSpace($case.invariant) -or $case.implementation -notmatch '^STEP-006[3-6]$') {
        throw "ecosystem threat case is incomplete: $($case.id)"
    }
}
if ($compat.schema -ne 'sico.ecosystem.compatibility-matrix.v0' -or @($compat.surfaces).Count -ne 12 -or @($compat.surfaces.id | Sort-Object -Unique).Count -ne 12) {
    throw 'compatibility matrix must contain 12 unique surfaces'
}
foreach ($surface in $compat.surfaces) {
    if ([string]::IsNullOrWhiteSpace($surface.identifier) -or [string]::IsNullOrWhiteSpace($surface.breaking_change) -or [string]::IsNullOrWhiteSpace($surface.unknown_behavior)) {
        throw "compatibility surface is incomplete: $($surface.id)"
    }
}
if ($adr -notmatch '(?m)^> - status: accepted\r?$' -or -not $adr.Contains('Registry, mirror, CDN, tag, channel and search results are untrusted')) {
    throw 'ADR-0006 is incomplete'
}
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or -not $rfc.Contains('registry-id / namespace / package-name / publisher-policy-id')) {
    throw 'RFC-0022 is incomplete'
}
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0062 is not complete' }
if ($plan -notmatch '(?m)^> - status: local-complete, blocked-external-evidence\r?$' -or -not $plan.Contains('Mobile platform gates')) {
    throw 'M7 completion disposition is incomplete'
}
if ($runner.status -ne 'blocked-external-runner' -or $runner.available.licensed_sdk -or $runner.available.adb) {
    throw 'M6 Android runner gate must remain blocked'
}
if ($environment.android.sdk -or $environment.android.ndk -or $environment.android.adb -or $environment.android.repository_buildable_host -or $environment.harmony.repository_plan) {
    throw 'platform recheck overstates mobile evidence'
}
if ($environment.rust.stable_toolchain -ne '1.97.0' -or $environment.rust.android_targets_installed) {
    throw 'platform recheck does not match the audited Rust environment'
}

Write-Output 'STEP_0062_OK threats=32 areas=8 compatibility_surfaces=12 trust=role-separated registry=untrusted mobile=deferred m6=blocked next=STEP-0063'
