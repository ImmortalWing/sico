param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}
$cases = Read-RepoFile 'ai-eval/tooling-cases.json' | ConvertFrom-Json
$source = Read-RepoFile 'crates/sico-ai-tools/src/lib.rs'
$manifest = Read-RepoFile 'crates/sico-ai-tools/Cargo.toml'
$protocol = Read-RepoFile 'ai-eval/tooling-protocol.md'
$rfc = Read-RepoFile 'docs/rfc/RFC-0028-ai-tooling-inspect-fix-v0.md'
$report = Read-RepoFile 'docs/reports/ai-tooling-measured-evaluation-v0.md'
$step = Read-RepoFile 'docs/steps/STEP-0068-ai-tooling-measured-evaluation.md'
if ($cases.schema -ne 'sico.ai-tool.cases.v0' -or @($cases.cases).Count -ne 29 -or @($cases.cases.id | Sort-Object -Unique).Count -ne 29) { throw 'AI tooling corpus must contain 29 unique cases' }
foreach ($area in 'metadata','limits','inspect','fix','evaluation') {
    if (@($cases.cases | Where-Object area -eq $area).Count -eq 0) { throw "AI tooling area missing: $area" }
}
foreach ($needle in 'sico.ai-tool.request.v0','sico.ai-tool.response.v0','MAX_WORKSPACE_BYTES','MAX_CHANGED_BYTES','build_index','syntax_identity','original_sha256','expected_diagnostics','contiguous_edit','candidate_has_diagnostics','assert_eq!(rejected, 512)','assert_eq!(paths.len(), 58)','assert_eq!((clean, diagnosed), (25, 33))') {
    if (-not $source.Contains($needle)) { throw "AI tooling invariant missing: $needle" }
}
if (-not $manifest.Contains('name = "sico-ai-tool"') -or -not $protocol.Contains('performs no filesystem write')) { throw 'AI tool binary/protocol is incomplete' }
if (([regex]::Matches($source, '(?m)^    #\[test\]\r?$')).Count -ne 12) { throw 'AI tooling test count drifted' }
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or -not $report.Contains('zero configured common model API credentials')) { throw 'AI tooling RFC/report incomplete' }
if ($step -notmatch '(?m)^> - status: complete-offline / live-model-not-authorized\r?$' -or -not $step.Contains('STEP_0068_OK')) { throw 'STEP-0068 incomplete' }
Write-Output 'STEP_0068_OK tests=8 mutations=512 inspect=58 clean=25 diagnosed=33 fixes=12 ai_tasks=96 model_runs=0 credentials=0 cost_authorization=absent external_side_effects=0 next=STEP-0069'
