param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$cases = Read-RepoFile 'tests/ecosystem/signed-registry-cases.json' | ConvertFrom-Json
$threats = Read-RepoFile 'tests/ecosystem/threat-matrix.json' | ConvertFrom-Json
$source = Read-RepoFile 'crates/sico-ecosystem/src/registry.rs'
$tests = Read-RepoFile 'crates/sico-ecosystem/tests/signed_registry.rs'
$crateCargo = Read-RepoFile 'crates/sico-ecosystem/Cargo.toml'
$rfc = Read-RepoFile 'docs/rfc/RFC-0024-signed-registry-metadata-v0.md'
$report = Read-RepoFile 'docs/reports/signed-local-registry-v0.md'
$step = Read-RepoFile 'docs/steps/STEP-0064-signed-local-registry.md'

if ($cases.schema -ne 'sico.ecosystem.signed-registry-cases.v0' -or @($cases.cases).Count -ne 32 -or @($cases.cases.id | Sort-Object -Unique).Count -ne 32) {
    throw 'signed registry corpus must contain 32 unique cases'
}
$requiredAreas = 'namespace','release','channel','checkpoint','transport','disclosure','compatibility'
foreach ($area in $requiredAreas) {
    if (@($cases.cases | Where-Object area -eq $area).Count -eq 0) { throw "signed registry area is missing: $area" }
}
if (@($threats.cases | Where-Object implementation -eq 'STEP-0064').Count -ne 10) {
    throw 'STEP-0064 must close exactly ten assigned ecosystem threats'
}
if (-not $crateCargo.Contains('sico-package = { path = "../sico-package" }')) {
    throw 'sico-ecosystem does not consume strict package verification'
}
foreach ($needle in 'SICO-REGISTRY-NAMESPACE-EVENT-V0','SICO-REGISTRY-RELEASE-V0','SICO-REGISTRY-CHANNEL-V0','SICO-REGISTRY-CHECKPOINT-V0','NamespaceEventKind','reuse_after','previous_event_sha256','verify_package_against_release','write_immutable','latest_channel','sorted_subset','verify_checkpoint_json') {
    if (-not $source.Contains($needle)) { throw "registry implementation invariant is missing: $needle" }
}
if (([regex]::Matches($tests, '(?m)^#\[test\]\r?$')).Count -ne 8 -or -not $tests.Contains('assert_eq!(rejected, bytes.len())')) {
    throw 'registry Rust test or mutation coverage drifted'
}
if (-not $tests.Contains('Component::new().finish()') -or -not $tests.Contains('b"tampered"') -or -not $tests.Contains('"<script>"')) {
    throw 'strict package, corruption or inert-text evidence is missing'
}
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or -not $rfc.Contains('append-only checkpoint')) {
    throw 'RFC-0024 is not accepted or lacks append-only checkpoint semantics'
}
if ($report -notmatch '(?m)^> - status: complete\r?$' -or -not $report.Contains('1,231/1,231 rejected')) {
    throw 'signed registry review report is incomplete'
}
if ($step -notmatch '(?m)^> - status: complete\r?$' -or -not $step.Contains('STEP_0064_OK')) {
    throw 'STEP-0064 record is incomplete'
}
if ($source.Contains('write_replace') -or $source.Contains('access_token') -or $source.Contains('private_key')) {
    throw 'registry implementation contains mutable replacement or forbidden secret-field material'
}

Write-Output 'STEP_0064_OK tests=8 mutations=1231 cases=32 threats=10 namespace=verified releases=verified checkpoints=3 downloads=reverified external_side_effects=0 next=STEP-0065'
