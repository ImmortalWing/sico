param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Read-Utf8([string]$Path) {
  return Get-Content -LiteralPath $Path -Raw -Encoding UTF8
}

function Read-Metadata([string]$Text, [string]$Name, [string]$Context) {
  $match = [regex]::Match($Text, "(?m)^> - $([regex]::Escape($Name)): ([^\r\n]+)$")
  if (-not $match.Success) {
    throw "$Context is missing metadata '$Name'"
  }
  return $match.Groups[1].Value.Trim()
}

function Invoke-CheckedScript([string]$Path, [string[]]$Arguments = @()) {
  $global:LASTEXITCODE = 0
  & $Path @Arguments
  if ($LASTEXITCODE -ne 0) {
    throw "validator failed: $Path"
  }
}

$root = (Resolve-Path $RepositoryRoot).Path

$global:LASTEXITCODE = 0
& (Join-Path $root 'tools/measure-syntax-candidates.ps1') -RepositoryRoot $root -CheckPath (Join-Path $root 'syntax-candidates/metrics-v1.json')
if ($LASTEXITCODE -ne 0) {
  throw 'syntax metric validation failed'
}
foreach ($name in @(
  'validate-semantic-cases.ps1',
  'validate-syntax-mutations.ps1',
  'validate-ai-eval.ps1',
  'test-ai-eval.ps1',
  'validate-error-taxonomy.ps1',
  'validate-diagnostics.ps1',
  'validate-semantic-query.ps1'
)) {
  Invoke-CheckedScript (Join-Path $root "tools/$name")
}

$stepDirectory = Join-Path $root 'docs/steps'
$stepFiles = @(Get-ChildItem -LiteralPath $stepDirectory -Filter 'STEP-*.md' | Sort-Object Name)
if ($stepFiles.Count -ne 14) {
  throw "expected 14 M0 step records, found $($stepFiles.Count)"
}
for ($number = 1; $number -le 14; $number++) {
  $prefix = "STEP-{0:D4}-" -f $number
  $matches = @($stepFiles | Where-Object Name -like "$prefix*")
  if ($matches.Count -ne 1) {
    throw "expected one step record for $prefix, found $($matches.Count)"
  }
  $status = Read-Metadata (Read-Utf8 $matches[0].FullName) 'status' $matches[0].Name
  if ($status -ne 'complete') {
    throw "$($matches[0].Name) is not complete: $status"
  }
}

$expectedDecisions = [ordered]@{
  'docs/adr/ADR-0001-sico-host-terminology.md' = 'accepted'
  'docs/adr/ADR-0002-runtime-platform-baseline.md' = 'accepted'
  'docs/rfc/RFC-0001-diagnostics-protocol-v0.md' = 'accepted'
  'docs/rfc/RFC-0002-semantic-index-query-v0.md' = 'accepted'
  'docs/rfc/RFC-0003-numeric-representation-v0.md' = 'proposed'
  'docs/rfc/RFC-0004-resource-async-mapping-v0.md' = 'proposed'
  'docs/rfc/RFC-0005-labeled-block-syntax-baseline.md' = 'accepted'
}
foreach ($relative in $expectedDecisions.Keys) {
  $fullPath = Join-Path $root $relative
  if (-not (Test-Path -LiteralPath $fullPath)) {
    throw "missing decision record: $relative"
  }
  $actual = Read-Metadata (Read-Utf8 $fullPath) 'status' $relative
  if ($actual -ne $expectedDecisions[$relative]) {
    throw "unexpected status for $($relative): expected $($expectedDecisions[$relative]), found $actual"
  }
}

$roadmap = Read-Utf8 (Join-Path $root 'docs/ROADMAP.md')
$missingCurrentPhase = $roadmap -notmatch '(?m)^> - current phase: M1\r?$'
$m0Start = $roadmap.IndexOf('## M0:', [StringComparison]::Ordinal)
$m1Start = $roadmap.IndexOf('## M1:', [StringComparison]::Ordinal)
if ($m0Start -lt 0 -or $m1Start -le $m0Start) {
  throw 'ROADMAP phase sections are missing or out of order'
}
$m0Section = $roadmap.Substring($m0Start, $m1Start - $m0Start)
$m0Header = $m0Section.Substring(0, [Math]::Min(160, $m0Section.Length))
$missingM0Complete = -not $m0Header.Contains('`complete`')
$missingM0Audit = $m0Section -notmatch '(?m)^\| [^|]+ \| complete \| .*STEP-0014'
if ($missingCurrentPhase -or $missingM0Complete -or $missingM0Audit) {
  throw 'ROADMAP does not record completed M0 and current M1'
}

$statusText = Read-Utf8 (Join-Path $root 'docs/STATUS.md')
foreach ($pattern in @(
  '(?m)^> - phase: M1 ',
  '(?m)^> - last completed step: STEP-0014$',
  '(?m)^> - next step: STEP-0015$'
)) {
  if ($statusText -notmatch $pattern) {
    throw "STATUS is missing expected state: $pattern"
  }
}

foreach ($required in @(
  'docs/reports/m0-exit-audit.md',
  'docs/plans/M1-compiler-frontend.md',
  'ai-eval/error-taxonomy.json'
)) {
  if (-not (Test-Path -LiteralPath (Join-Path $root $required))) {
    throw "missing M0 exit artifact: $required"
  }
}

$modelRuns = @(Get-ChildItem -LiteralPath (Join-Path $root 'ai-eval/runs') -File -Filter '*.json')
if ($modelRuns.Count -ne 0) {
  throw 'ai-eval/runs contains unreviewed JSON; M0 records no official model run'
}

$markdownFiles = @(Get-ChildItem -LiteralPath $root -Recurse -File -Filter '*.md' |
  Where-Object { $_.FullName -notmatch '[\\/]target[\\/]' })
$localLinks = 0
foreach ($file in $markdownFiles) {
  $base = Split-Path -Parent $file.FullName
  $content = Read-Utf8 $file.FullName
  foreach ($match in [regex]::Matches($content, '(?<!\!)\[[^\]]+\]\(([^)]+)\)')) {
    $target = $match.Groups[1].Value.Trim()
    if ($target.StartsWith('<') -and $target.EndsWith('>')) {
      $target = $target.Substring(1, $target.Length - 2)
    }
    if ($target -match '^(https?://|mailto:|#)') {
      continue
    }
    $pathOnly = ($target -split '#', 2)[0]
    if ([string]::IsNullOrWhiteSpace($pathOnly)) {
      continue
    }
    $resolved = Join-Path $base ([Uri]::UnescapeDataString($pathOnly))
    if (-not (Test-Path -LiteralPath $resolved)) {
      throw "broken Markdown link in $($file.FullName): $target"
    }
    $localLinks++
  }
}

Write-Output "M0_EXIT_DOCS_OK steps=14 decisions=7 markdown=$($markdownFiles.Count) local_links=$localLinks official_ai_runs=0 current_phase=M1 next=STEP-0015"
