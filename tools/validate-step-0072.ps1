param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$contract = Read-RepoFile 'tests/tooling/user-manual-contract.json' | ConvertFrom-Json
$rootReadme = Read-RepoFile 'README.md'
$development = Read-RepoFile 'docs/development/README.md'
$cliSource = Read-RepoFile 'crates/sico-cli/src/lib.rs'
$workspace = Read-RepoFile 'Cargo.toml'
$step = Read-RepoFile 'docs/steps/STEP-0072-user-manual-information-architecture.md'

if ($contract.schema -ne 'sico.user-manual-contract.v0' -or $contract.version -ne '0.0.1' -or $contract.root_readme_role -ne 'user-entry') { throw 'user manual contract identity drifted' }
if (@($contract.manuals).Count -ne 11 -or @($contract.manuals | Sort-Object -Unique).Count -ne 11) { throw 'user manual index must contain eleven unique files' }
foreach ($manual in $contract.manuals) {
    $null = Read-RepoFile (Join-Path 'docs/user-guide' $manual)
}
if (@($contract.cli_commands).Count -ne 6 -or @($contract.cli_commands | Sort-Object -Unique).Count -ne 6) { throw 'CLI command contract drifted' }
foreach ($command in $contract.cli_commands) {
    $implemented = 'Command::new("{0}")' -f $command
    if (-not $cliSource.Contains($implemented)) { throw "CLI implementation missing documented command: $command" }
    if (-not $rootReadme.Contains("sico $command")) { throw "root user entry missing command: $command" }
}
if (-not $workspace.Contains('version = "0.0.1"') -or -not $rootReadme.Contains('`v0.0.1`')) { throw 'documented version does not match workspace' }
if (-not $rootReadme.StartsWith('# Sico ') -or -not $rootReadme.Contains('docs/user-guide/README.md') -or -not $rootReadme.Contains('docs/development/README.md')) { throw 'root README is not the user entry' }
if (-not $development.StartsWith('# Sico ') -or -not $development.Contains('../../DEVELOPMENT.md') -or -not $development.Contains('../user-guide/README.md')) { throw 'development handbook migration is incomplete' }

$gettingStarted = Read-RepoFile 'docs/user-guide/GETTING-STARTED.md'
$trust = Read-RepoFile 'docs/user-guide/PACKAGES-AND-TRUST.md'
$editor = Read-RepoFile 'docs/user-guide/EDITOR.md'
$ai = Read-RepoFile 'docs/user-guide/AI-TOOLS.md'
$limits = Read-RepoFile 'docs/user-guide/LIMITATIONS.md'
foreach ($needle in 'function main() returns Int:','sico check hello.sico','sico run hello.sico','sico build --app-id','sico inspect hello.sapp','--allow-unsigned-dev') {
    if (-not $gettingStarted.Contains($needle)) { throw "quick start invariant missing: $needle" }
}
foreach ($needle in 'development-valid-untrusted','CSPRNG','production key','--trusted-key') {
    if (-not $trust.Contains($needle)) { throw "trust manual invariant missing: $needle" }
}
if (-not $editor.Contains('source debugger/DAP') -or -not $editor.Contains('sico.debug') -or -not $editor.Contains('-32004') -or -not $ai.Contains('sico-ai-tool') -or -not $ai.Contains('compiler-backed JSON')) { throw 'tooling evidence boundary drifted' }
foreach ($needle in 'blocked-not-implemented','proposed-not-implemented','contract-verified-not-runtime-verified') {
    if (-not $limits.Contains($needle)) { throw "limitations manual invariant missing: $needle" }
}
if ($contract.verified_quick_start.result -ne 42 -or -not $contract.verified_quick_start.unsigned_development -or -not $contract.verified_quick_start.development_signed -or $contract.evidence_boundaries.live_model_runs -ne 0) { throw 'quick-start evidence contract incomplete' }
if ($step -notmatch '(?m)^> - status: complete\r?$' -or -not $step.Contains('STEP_0072_OK')) { throw 'STEP-0072 record incomplete' }

$badLinks = @()
$markdownFiles = & rg --files -g '*.md' $root
foreach ($file in $markdownFiles) {
    $text = Get-Content -LiteralPath $file -Raw -Encoding UTF8
    foreach ($match in [regex]::Matches($text, '\[[^\]]*\]\(([^)]+)\)')) {
        $link = $match.Groups[1].Value
        if ($link -match '^(https?://|mailto:|#)' -or $link.Contains(' ')) { continue }
        $relative = ($link -split '#')[0]
        if ([string]::IsNullOrEmpty($relative)) { continue }
        if (-not (Test-Path -LiteralPath (Join-Path (Split-Path -Parent $file) $relative))) { $badLinks += "$file -> $link" }
    }
}
if ($badLinks.Count -ne 0) {
    $details = $badLinks -join [Environment]::NewLine
    throw "broken Markdown links:$([Environment]::NewLine)$details"
}

Write-Output 'STEP_0072_OK version=0.0.1 manuals=11 commands=6 quick_start=42 unsigned_dev=verified signed_dev=verified lsp=bounded ai=compiler-backed production=external-gated platforms=honest'
