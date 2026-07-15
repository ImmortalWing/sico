param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
. (Join-Path $PSScriptRoot 'ai-eval-common.ps1')

$root = (Resolve-Path $RepositoryRoot).Path
$protocol = Read-Utf8Json (Join-Path $root 'ai-eval/protocol.json')
if ($protocol.schema_version -ne 1 -or $protocol.protocol_id -ne 'sico-ai-eval-v1') {
  throw 'unsupported AI evaluation protocol'
}
if ($protocol.minimum_model_repetitions -ne 30) {
  throw 'v1 requires exactly 30 minimum model repetitions'
}

$tasks = @(Get-AiEvalTasks $root)
$expectedCounts = @{ generation = 30; understanding = 30; repair = 36 }
$categoryCounts = @{ generation = 0; understanding = 0; repair = 0 }
$syntaxCounts = @{ A0 = 0; B = 0; C = 0 }
$taskIds = @{}
$pairSyntax = @{}
$repairKeys = @{}

foreach ($task in $tasks) {
  if ($taskIds.ContainsKey($task.task_id)) {
    throw "duplicate task id: $($task.task_id)"
  }
  $taskIds[$task.task_id] = $true

  if (-not $categoryCounts.ContainsKey($task.category)) {
    throw "unknown category: $($task.category)"
  }
  if (-not $syntaxCounts.ContainsKey($task.syntax)) {
    throw "unknown syntax: $($task.syntax)"
  }
  $categoryCounts[$task.category] += 1
  $syntaxCounts[$task.syntax] += 1

  $guidePath = Join-Path $root $task.guide_path
  if (-not (Test-Path -LiteralPath $guidePath)) {
    throw "missing guide for $($task.task_id): $($task.guide_path)"
  }
  foreach ($path in @($task.input_path, $task.expected_path)) {
    if ($null -ne $path -and -not (Test-Path -LiteralPath (Join-Path $root $path))) {
      throw "missing task path for $($task.task_id): $path"
    }
  }

  $sourcePath = if ($task.category -eq 'understanding') { $task.input_path } else { $task.expected_path }
  if ($null -ne $sourcePath) {
    $sourceText = Read-NormalizedText (Join-Path $root $sourcePath)
    if ((Read-SicoMetadata $sourceText 'expect') -ne 'accept') {
      throw "task oracle is not an accept case: $($task.task_id)"
    }
    if ($task.category -in @('generation', 'understanding') -and
        (Read-SicoMetadata $sourceText 'case') -ne $task.case_id) {
      throw "task case metadata mismatch: $($task.task_id)"
    }
    if ($task.syntax -in @('B', 'C') -and (Read-SicoMetadata $sourceText 'syntax') -ne $task.syntax) {
      throw "task syntax metadata mismatch: $($task.task_id)"
    }
  }

  if ($task.category -in @('generation', 'understanding')) {
    $key = "$($task.pair_id)|$($task.syntax)"
    if ($pairSyntax.ContainsKey($key)) {
      throw "duplicate pair/syntax: $key"
    }
    $pairSyntax[$key] = $true
  }
  else {
    $repairKey = "$($task.case_id)|$($task.syntax)"
    if ($repairKeys.ContainsKey($repairKey)) {
      throw "duplicate repair mutation/syntax: $repairKey"
    }
    $repairKeys[$repairKey] = $true
  }
}

foreach ($category in $expectedCounts.Keys) {
  if ($categoryCounts[$category] -ne $expectedCounts[$category]) {
    throw "expected $($expectedCounts[$category]) $category tasks, found $($categoryCounts[$category])"
  }
}
foreach ($syntax in @('A0', 'B', 'C')) {
  if ($syntaxCounts[$syntax] -ne 32) {
    throw "expected 32 $syntax tasks, found $($syntaxCounts[$syntax])"
  }
}

$pairGroups = $tasks | Where-Object { $_.category -in @('generation', 'understanding') } | Group-Object pair_id
foreach ($group in $pairGroups) {
  $syntaxes = @($group.Group.syntax | Sort-Object -Unique)
  if ($syntaxes.Count -ne 3 -or ($syntaxes -join ',') -ne 'A0,B,C') {
    throw "pair does not cover A0/B/C: $($group.Name)"
  }
}

$mutationManifest = Read-Utf8Json (Join-Path $root 'syntax-mutations/manifest.json')
$mutationEntries = @{}
foreach ($entry in @($mutationManifest.entries)) {
  $mutationEntries["$($entry.mutation)|$($entry.syntax)"] = $entry
}
foreach ($task in @($tasks | Where-Object category -eq 'repair')) {
  $key = "$($task.case_id)|$($task.syntax)"
  if (-not $mutationEntries.ContainsKey($key)) {
    throw "repair task has no mutation manifest entry: $($task.task_id)"
  }
  $entry = $mutationEntries[$key]
  if ($task.input_path -ne $entry.mutant -or $task.expected_path -ne $entry.source -or $task.diagnostic -ne $entry.diagnostic) {
    throw "repair task drifted from mutation manifest: $($task.task_id)"
  }
}
foreach ($key in $mutationEntries.Keys) {
  if (-not $repairKeys.ContainsKey($key)) {
    throw "mutation manifest entry is missing a repair task: $key"
  }
}

Write-Output 'AI_EVAL_DATASET_OK protocol=v1 tasks=96 generation=30 understanding=30 repair=36 A0=32 B=32 C=32'
