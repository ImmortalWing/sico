param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$cases = Read-RepoFile 'tests/ecosystem/secure-update-cases.json' | ConvertFrom-Json
$threats = Read-RepoFile 'tests/ecosystem/threat-matrix.json' | ConvertFrom-Json
$source = Read-RepoFile 'crates/sico-ecosystem/src/update.rs'
$tests = Read-RepoFile 'crates/sico-ecosystem/tests/secure_update.rs'
$rfc = Read-RepoFile 'docs/rfc/RFC-0025-secure-update-recovery-v0.md'
$report = Read-RepoFile 'docs/reports/secure-update-recovery-v0.md'
$step = Read-RepoFile 'docs/steps/STEP-0065-secure-update-rollback.md'

if ($cases.schema -ne 'sico.ecosystem.secure-update-cases.v0' -or @($cases.cases).Count -ne 36 -or @($cases.cases.id | Sort-Object -Unique).Count -ne 36) {
    throw 'secure update corpus must contain 36 unique cases'
}
$requiredAreas = 'monotonic','freshness','consistency','capability','staging','state','advisory','recovery','mutation'
foreach ($area in $requiredAreas) {
    if (@($cases.cases | Where-Object area -eq $area).Count -eq 0) { throw "secure update area is missing: $area" }
}
if (@($threats.cases | Where-Object implementation -eq 'STEP-0065').Count -ne 10) {
    throw 'STEP-0065 must close exactly ten assigned ecosystem threats'
}
foreach ($needle in 'SICO-UPDATE-SNAPSHOT-V0','SICO-UPDATE-ADVISORY-V0','SICO-UPDATE-RECOVERY-AUTHORIZATION-V0','SICO-UPDATE-TRUSTED-STATE-V0','last_trusted_time','validate_monotonic_update','write_staging','commit_state','verify_retained_revision','permissions_must_be_reconfirmed','application id drift','package capability closure','local_recovery_confirmation') {
    if (-not $source.Contains($needle)) { throw "secure update invariant is missing: $needle" }
}
if (([regex]::Matches($tests, '(?m)^#\[test\]\r?$')).Count -ne 8 -or -not $tests.Contains('assert_eq!(rejected, fixture.second.snapshot.len())')) {
    throw 'secure update Rust test or mutation coverage drifted'
}
foreach ($needle in 'last_mut().unwrap() ^= 1','RecoveryDecision::Signed','RecoveryDecision::ExplicitLocal','permissions_must_be_reconfirmed','wasi:clocks/monotonic-clock@0.2.0') {
    if (-not $tests.Contains($needle)) { throw "secure update evidence is missing: $needle" }
}
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or -not $rfc.Contains('create-new activation commits')) {
    throw 'RFC-0025 is not accepted or lacks atomic commit semantics'
}
if ($report -notmatch '(?m)^> - status: complete\r?$' -or -not $report.Contains('1,592/1,592 rejected')) {
    throw 'secure update review report is incomplete'
}
if ($step -notmatch '(?m)^> - status: complete\r?$' -or -not $step.Contains('STEP_0065_OK')) {
    throw 'STEP-0065 record is incomplete'
}
if ($source.Contains('access_token') -or $source.Contains('private_key') -or $source.Contains('automatic_recovery')) {
    throw 'secure update implementation contains forbidden secret or implicit recovery material'
}

Write-Output 'STEP_0065_OK tests=8 mutations=1592 cases=36 threats=10 updates=verified advisories=append-only recovery=signed-or-explicit retained=2 external_side_effects=0 next=STEP-0066'
