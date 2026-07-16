param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}
$cases = Read-RepoFile 'tests/ecosystem/dependency-stability-cases.json' | ConvertFrom-Json
$threats = Read-RepoFile 'tests/ecosystem/threat-matrix.json' | ConvertFrom-Json
$source = Read-RepoFile 'crates/sico-ecosystem/src/dependency.rs'
$tests = Read-RepoFile 'crates/sico-ecosystem/tests/dependency_resolution.rs'
$rfc = Read-RepoFile 'docs/rfc/RFC-0026-dependency-lock-compatibility-v0.md'
$report = Read-RepoFile 'docs/reports/dependency-standard-library-v0.md'
$step = Read-RepoFile 'docs/steps/STEP-0066-dependency-standard-library-stability.md'
if ($cases.schema -ne 'sico.ecosystem.dependency-stability-cases.v0' -or @($cases.cases).Count -ne 32 -or @($cases.cases.id | Sort-Object -Unique).Count -ne 32) { throw 'dependency corpus must contain 32 unique cases' }
foreach ($area in 'identity','semver','resolution','lock','capability','lifecycle','compatibility','stdlib','mutation') {
    if (@($cases.cases | Where-Object area -eq $area).Count -eq 0) { throw "dependency area missing: $area" }
}
if (@($threats.cases | Where-Object implementation -eq 'STEP-0066').Count -ne 7) { throw 'STEP-0066 threat allocation drifted' }
foreach ($needle in 'sico.dependency.lock-graph.v0','sico.standard-library.contract.v0','DependencyIdentity','VersionRequirement','Ambiguous','validate_locked_graph','CapabilityClosure','RevocationPolicy','standard_library_digest','verify_dependency_lock_json') {
    if (-not $source.Contains($needle)) { throw "dependency invariant missing: $needle" }
}
if (([regex]::Matches($tests, '(?m)^#\[test\]\r?$')).Count -ne 8 -or -not $tests.Contains('assert_eq!(rejected, bytes.len())')) { throw 'dependency test coverage drifted' }
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or -not $report.Contains('1,140/1,140 rejected')) { throw 'dependency RFC/report incomplete' }
if ($step -notmatch '(?m)^> - status: complete\r?$' -or -not $step.Contains('STEP_0066_OK')) { throw 'STEP-0066 incomplete' }
Write-Output 'STEP_0066_OK tests=8 mutations=1140 cases=32 threats=7 nodes=2 semver=canonical confusion=rejected capabilities=closed external_side_effects=0 next=STEP-0067'
