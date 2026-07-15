param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Read-Json([string]$Path) {
  return Get-Content -LiteralPath $Path -Raw -Encoding UTF8 | ConvertFrom-Json
}

function Assert-NonEmptyString([object]$Value, [string]$Context) {
  if ($Value -isnot [string] -or [string]::IsNullOrWhiteSpace([string]$Value)) {
    throw "$Context must be a non-empty string"
  }
}

$root = (Resolve-Path $RepositoryRoot).Path
$taxonomy = Read-Json (Join-Path $root 'ai-eval/error-taxonomy.json')
$catalog = Read-Json (Join-Path $root 'diagnostics/catalog.json')
$caseMap = Read-Json (Join-Path $root 'diagnostics/semantic-case-map.json')
$mutations = Read-Json (Join-Path $root 'syntax-mutations/manifest.json')
$issuesText = Get-Content -LiteralPath (Join-Path $root 'examples/ISSUES.md') -Raw -Encoding UTF8

if ($taxonomy.schema -ne 'sico.ai-error-taxonomy.v0' -or $taxonomy.status -ne 'designed-corpus-taxonomy') {
  throw 'invalid taxonomy header'
}
if ($taxonomy.observed_ai_frequency -ne 'not-measured' -or $taxonomy.ranking_allowed -ne $false) {
  throw 'taxonomy must not claim measured AI frequency or ranking'
}

$classes = @($taxonomy.classes)
if ($classes.Count -ne 12) {
  throw "expected 12 taxonomy classes, found $($classes.Count)"
}

$classIds = @{}
$diagnosticOwners = @{}
$mutationOwners = @{}
$referencedIssues = @{}
foreach ($class in $classes) {
  Assert-NonEmptyString $class.id 'class.id'
  if ([string]$class.id -notmatch '^[a-z][a-z0-9-]+$') {
    throw "invalid class id: $($class.id)"
  }
  if ($classIds.ContainsKey([string]$class.id)) {
    throw "duplicate class id: $($class.id)"
  }
  $classIds[[string]$class.id] = $true
  foreach ($field in @('failure_signal', 'compiler_prevention', 'ai_repair_constraint')) {
    Assert-NonEmptyString $class.$field "$($class.id).$field"
  }
  foreach ($key in @($class.diagnostic_keys)) {
    if ($diagnosticOwners.ContainsKey([string]$key)) {
      throw "diagnostic key has multiple classes: $key"
    }
    $diagnosticOwners[[string]$key] = [string]$class.id
  }
  foreach ($mutation in @($class.mutation_ids)) {
    if ($mutationOwners.ContainsKey([string]$mutation)) {
      throw "mutation id has multiple classes: $mutation"
    }
    $mutationOwners[[string]$mutation] = [string]$class.id
  }
  foreach ($issue in @($class.related_issue_ids)) {
    if ([string]$issue -notmatch '^[A-Z]+[0-9]{2}$') {
      throw "invalid related issue id: $issue"
    }
    if ($issuesText -notmatch "(?m)^\| $([regex]::Escape([string]$issue)) \|") {
      throw "taxonomy references missing issue id: $issue"
    }
    $referencedIssues[[string]$issue] = $true
  }
}

$catalogKeys = @($catalog.diagnostics | ForEach-Object { [string]$_.key } | Sort-Object -Unique)
$taxonomyKeys = @($diagnosticOwners.Keys | Sort-Object)
if (($catalogKeys -join "`n") -cne ($taxonomyKeys -join "`n")) {
  throw 'taxonomy diagnostic keys do not exactly cover the v0 catalog'
}

$caseIds = @{}
foreach ($case in @($caseMap.cases)) {
  if ($caseIds.ContainsKey([string]$case.case)) {
    throw "duplicate semantic case in diagnostic map: $($case.case)"
  }
  $caseIds[[string]$case.case] = $true
  if (-not $diagnosticOwners.ContainsKey([string]$case.key)) {
    throw "semantic case has no taxonomy class: $($case.case)"
  }
}
if ($caseIds.Count -ne 29) {
  throw "expected 29 semantic invalid cases, found $($caseIds.Count)"
}

$manifestMutationIds = @($mutations.entries | ForEach-Object { [string]$_.mutation } | Sort-Object -Unique)
$taxonomyMutationIds = @($mutationOwners.Keys | Sort-Object)
if (($manifestMutationIds -join "`n") -cne ($taxonomyMutationIds -join "`n")) {
  throw 'taxonomy mutation ids do not exactly cover the mutation manifest'
}
if (@($mutations.entries).Count -ne 36) {
  throw "expected 36 syntax mutation variants, found $(@($mutations.entries).Count)"
}

Write-Output "ERROR_TAXONOMY_OK classes=$($classes.Count) diagnostics=$($taxonomyKeys.Count) semantic_cases=$($caseIds.Count) mutation_intents=$($taxonomyMutationIds.Count) mutation_variants=$(@($mutations.entries).Count) related_issues=$($referencedIssues.Count) frequency=not-measured"
