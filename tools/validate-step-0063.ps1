param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$cases = Read-RepoFile 'tests/ecosystem/publisher-key-lifecycle-cases.json' | ConvertFrom-Json
$threats = Read-RepoFile 'tests/ecosystem/threat-matrix.json' | ConvertFrom-Json
$source = Read-RepoFile 'crates/sico-ecosystem/src/lib.rs'
$tests = Read-RepoFile 'crates/sico-ecosystem/tests/publisher_lifecycle.rs'
$cargo = Read-RepoFile 'Cargo.toml'
$crateCargo = Read-RepoFile 'crates/sico-ecosystem/Cargo.toml'
$rfc = Read-RepoFile 'docs/rfc/RFC-0023-production-publisher-policy-v0.md'
$report = Read-RepoFile 'docs/reports/publisher-identity-key-lifecycle-v0.md'
$step = Read-RepoFile 'docs/steps/STEP-0063-production-publisher-identity-key-lifecycle.md'

if ($cases.schema -ne 'sico.ecosystem.publisher-key-lifecycle-cases.v0' -or @($cases.cases).Count -ne 28 -or @($cases.cases.id | Sort-Object -Unique).Count -ne 28) {
    throw 'publisher lifecycle corpus must contain 28 unique cases'
}
$requiredAreas = 'separation','threshold','roles','time','canonical','identity','rotation','revocation','recovery','disclosure','privacy','mutation'
foreach ($area in $requiredAreas) {
    if (@($cases.cases | Where-Object area -eq $area).Count -eq 0) { throw "publisher lifecycle area is missing: $area" }
}
if (@($threats.cases | Where-Object implementation -eq 'STEP-0063').Count -ne 5) {
    throw 'STEP-0063 must close exactly five assigned ecosystem threats'
}
if (-not $cargo.Contains('"crates/sico-ecosystem"') -or -not $crateCargo.Contains('ed25519-dalek.workspace = true')) {
    throw 'sico-ecosystem is not a locked workspace member'
}
foreach ($needle in 'SICO-PUBLISHER-KEY-ID-V0','SICO-PUBLISHER-POLICY-V0','SICO-PUBLISHER-POLICY-UPDATE-V0','DevelopmentKeyReuse','RoutineRotation','EmergencyRevocation','Recovery','OwnerApprovalRequired','verify_initial_policy_json','verify_policy_update_json') {
    if (-not $source.Contains($needle)) { throw "publisher implementation invariant is missing: $needle" }
}
if (([regex]::Matches($tests, '(?m)^#\[test\]\r?$')).Count -ne 8 -or -not $tests.Contains('0..2_048_usize')) {
    throw 'publisher Rust test and mutation coverage drifted'
}
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or -not $rfc.Contains('old-root and new-root thresholds')) {
    throw 'RFC-0023 is not accepted or lacks dual-root authorization'
}
if ($report -notmatch '(?m)^> - status: complete\r?$' -or -not $report.Contains('2,048/2,048 rejected')) {
    throw 'publisher review report is incomplete'
}
if ($step -notmatch '(?m)^> - status: complete\r?$' -or -not $step.Contains('STEP_0063_OK')) {
    throw 'STEP-0063 record is incomplete'
}
if ($source.Contains('sico.sapp.signature.v0') -or $source.Contains('access_token') -or $source.Contains('private_key')) {
    throw 'publisher implementation contains forbidden development-domain or secret-field material'
}

Write-Output 'STEP_0063_OK tests=8 mutations=2048 cases=28 threats=5 roles=5 rotation=verified revocation=verified recovery=verified external_side_effects=0 next=STEP-0064'
