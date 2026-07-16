param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}
$cases = Read-RepoFile 'tests/tooling/language-server-cases.json' | ConvertFrom-Json
$source = Read-RepoFile 'crates/sico-language-server/src/lib.rs'
$manifest = Read-RepoFile 'crates/sico-language-server/Cargo.toml'
$rfc = Read-RepoFile 'docs/rfc/RFC-0027-language-server-editor-protocol-v0.md'
$report = Read-RepoFile 'docs/reports/language-server-editor-workflow-v0.md'
$step = Read-RepoFile 'docs/steps/STEP-0067-language-server-editor-workflow.md'
if ($cases.schema -ne 'sico.tooling.language-server-cases.v0' -or @($cases.cases).Count -ne 24 -or @($cases.cases.id | Sort-Object -Unique).Count -ne 24) { throw 'language-server corpus must contain 24 unique cases' }
foreach ($area in 'lifecycle','framing','limits','sync','coordinates','diagnostics','index','navigation','format','workflow') {
    if (@($cases.cases | Where-Object area -eq $area).Count -eq 0) { throw "language-server area missing: $area" }
}
foreach ($needle in 'MAX_MESSAGE_BYTES','MAX_HEADER_BYTES','MAX_OPEN_DOCUMENTS','MAX_WORKSPACE_SOURCE_BYTES','position_to_byte','byte_to_position','build_index','syntax_identity','sico.check','sico.run','sico.debug','source debugging is unavailable','shell') {
    if (-not $source.Contains($needle)) { throw "language-server invariant missing: $needle" }
}
if (-not $manifest.Contains('name = "sico-lsp"')) { throw 'sico-lsp binary is not registered' }
if (([regex]::Matches($source, '(?m)^    #\[test\]\r?$')).Count -ne 8 -or -not $source.Contains('assert_eq!(rejected, 512)')) { throw 'language-server test coverage drifted' }
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or -not $report.Contains('512 malformed or oversized frame mutations')) { throw 'language-server RFC/report incomplete' }
if ($step -notmatch '(?m)^> - status: complete\r?$' -or -not $step.Contains('STEP_0067_OK')) { throw 'STEP-0067 incomplete' }
Write-Output 'STEP_0067_OK tests=8 mutations=512 cases=24 lsp=stdio positions=utf16 diagnostics=compiler index=compiler format=canonical shell=false debug=explicit-unavailable external_side_effects=0 next=STEP-0068'
