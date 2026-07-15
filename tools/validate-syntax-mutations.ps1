param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'

function Read-NormalizedText([string]$Path) {
  $text = [IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8)
  return $text.Replace("`r`n", "`n").Replace("`r", "`n")
}

function Remove-Metadata([string]$Text) {
  $lines = $Text -split "`n"
  $kept = foreach ($line in $lines) {
    if ($line -notmatch '^// (case|syntax|expect|semantics|mutation|source-case|recovery):') {
      $line
    }
  }
  return $kept -join "`n"
}

function Read-Header([string]$Text, [string]$Name) {
  $match = [regex]::Match($Text, "(?m)^// $([regex]::Escape($Name)): (.+)$")
  if (-not $match.Success) {
    throw "missing metadata '$Name'"
  }
  return $match.Groups[1].Value
}

function Count-Occurrences([string]$Text, [string]$Fragment) {
  if ($Fragment.Length -eq 0) {
    throw 'source_fragment cannot be empty'
  }

  $count = 0
  $offset = 0
  while ($true) {
    $index = $Text.IndexOf($Fragment, $offset, [StringComparison]::Ordinal)
    if ($index -lt 0) {
      return $count
    }
    $count += 1
    $offset = $index + $Fragment.Length
  }
}

$root = (Resolve-Path $RepositoryRoot).Path
$manifestPath = Join-Path $root 'syntax-mutations/manifest.json'
$manifest = Get-Content -LiteralPath $manifestPath -Encoding UTF8 -Raw | ConvertFrom-Json

if ($manifest.schema_version -ne 1) {
  throw "unsupported manifest schema: $($manifest.schema_version)"
}

$entries = @($manifest.entries)
if ($entries.Count -ne 36) {
  throw "expected 36 entries, found $($entries.Count)"
}

$keys = @{}
$registeredMutants = @{}
$syntaxCounts = @{ A0 = 0; B = 0; C = 0 }
$mutationCounts = @{
  'MUT-001' = 0
  'MUT-002' = 0
  'MUT-003' = 0
  'MUT-004' = 0
  'MUT-005' = 0
  'MUT-006' = 0
  'MUT-007' = 0
  'MUT-008' = 0
  'MUT-009' = 0
  'MUT-010' = 0
  'MUT-011' = 0
  'MUT-012' = 0
}

foreach ($entry in $entries) {
  $key = "$($entry.mutation)|$($entry.syntax)"
  if ($keys.ContainsKey($key)) {
    throw "duplicate manifest key: $key"
  }
  $keys[$key] = $true

  if (-not $syntaxCounts.ContainsKey([string]$entry.syntax)) {
    throw "unknown syntax: $($entry.syntax)"
  }
  $syntaxCounts[[string]$entry.syntax] += 1
  if (-not $mutationCounts.ContainsKey([string]$entry.mutation)) {
    throw "unknown mutation: $($entry.mutation)"
  }
  $mutationCounts[[string]$entry.mutation] += 1

  $sourcePath = Join-Path $root $entry.source
  $mutantPath = Join-Path $root $entry.mutant
  if (-not (Test-Path -LiteralPath $sourcePath)) {
    throw "missing source: $($entry.source)"
  }
  if (-not (Test-Path -LiteralPath $mutantPath)) {
    throw "missing mutant: $($entry.mutant)"
  }
  $registeredMutants[(Resolve-Path $mutantPath).Path] = $true

  $sourceRaw = Read-NormalizedText $sourcePath
  $mutantRaw = Read-NormalizedText $mutantPath

  if ((Read-Header $sourceRaw 'case') -ne $entry.source_case) {
    throw "source case mismatch: $key"
  }
  if ((Read-Header $sourceRaw 'expect') -ne 'accept') {
    throw "mutation source is not an accept case: $key"
  }
  if ((Read-Header $mutantRaw 'mutation') -ne $entry.mutation) {
    throw "mutant id mismatch: $key"
  }
  if ((Read-Header $mutantRaw 'syntax') -ne $entry.syntax) {
    throw "mutant syntax mismatch: $key"
  }
  if ((Read-Header $mutantRaw 'source-case') -ne $entry.source_case) {
    throw "mutant source case mismatch: $key"
  }
  if ((Read-Header $mutantRaw 'expect') -ne "reject($($entry.diagnostic))") {
    throw "mutant diagnostic mismatch: $key"
  }
  if ((Read-Header $mutantRaw 'recovery') -ne $entry.recovery) {
    throw "mutant recovery mismatch: $key"
  }

  $sourceBody = Remove-Metadata $sourceRaw
  $mutantBody = Remove-Metadata $mutantRaw
  $occurrences = Count-Occurrences $sourceBody $entry.source_fragment
  if ($occurrences -ne 1) {
    throw "source fragment occurrence count for $key is $occurrences, expected 1"
  }

  $index = $sourceBody.IndexOf($entry.source_fragment, [StringComparison]::Ordinal)
  $expectedBody = $sourceBody.Substring(0, $index) +
    $entry.replacement +
    $sourceBody.Substring($index + $entry.source_fragment.Length)

  if ($expectedBody -cne $mutantBody) {
    throw "mutant body is not the declared single replacement: $key"
  }
}

foreach ($syntax in @('A0', 'B', 'C')) {
  if ($syntaxCounts[$syntax] -ne 12) {
    throw "expected 12 entries for $syntax, found $($syntaxCounts[$syntax])"
  }
}

foreach ($mutation in $mutationCounts.Keys) {
  if ($mutationCounts[$mutation] -ne 3) {
    throw "expected 3 entries for $mutation, found $($mutationCounts[$mutation])"
  }
}

$diskMutants = Get-ChildItem (Join-Path $root 'syntax-mutations') -Recurse -Filter '*.sico'
if ($diskMutants.Count -ne 36) {
  throw "expected 36 mutant files on disk, found $($diskMutants.Count)"
}
foreach ($file in $diskMutants) {
  if (-not $registeredMutants.ContainsKey($file.FullName)) {
    throw "unregistered mutant file: $($file.FullName)"
  }
}

Write-Output 'MUTATION_CORPUS_OK entries=36 A0=12 B=12 C=12'
