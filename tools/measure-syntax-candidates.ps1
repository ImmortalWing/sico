param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$OutputPath,
  [string]$CheckPath
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Read-NormalizedText([string]$Path) {
  $text = [IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8)
  return $text.Replace("`r`n", "`n").Replace("`r", "`n")
}

function Remove-Metadata([string]$Text) {
  $kept = foreach ($line in ($Text -split "`n")) {
    if ($line -notmatch '^// (case|syntax|expect|semantics):' -and $line -notmatch '^\s*$') {
      $line
    }
  }
  return ($kept -join "`n")
}

function Get-Cohort([string]$RelativePath) {
  $group = ($RelativePath -split '/')[0]
  if ($group -in @('numbers-units', 'nominal-invariants', 'exhaustive-match', 'result-mapping')) {
    return 'round-1'
  }
  return 'round-2'
}

function Measure-Source([string]$Text, [string]$Syntax) {
  $source = Remove-Metadata $Text
  $nonEmptyLines = @($source -split "`n" | Where-Object { $_ -notmatch '^\s*$' })
  $lexicalPattern = '"(?:\\.|[^"\\])*"|\d+(?:\.\d+)?|[A-Za-z_][A-Za-z0-9_]*|->|=>|==|!=|<=|>=|::|[{}\[\](),.:?@+*/=<>-]'
  $punctuationPattern = '^(?:->|=>|==|!=|<=|>=|::|[{}\[\](),.:?@+*/=<>-])$'
  $tokens = @([regex]::Matches($source, $lexicalPattern) | ForEach-Object { $_.Value })
  $punctuation = @($tokens | Where-Object { $_ -match $punctuationPattern })
  if ($Syntax -eq 'A0') {
    $blockClosers = [regex]::Matches($source, '(?m)^\s*end(?:\)|$)').Count
    $labeledClosers = 0
  }
  elseif ($Syntax -eq 'B') {
    $blockClosers = [regex]::Matches($source, '(?m)^\s*end (?:enum|record|function|match|capability|resource|interface|using|task)\b').Count
    $labeledClosers = $blockClosers
  }
  else {
    $blockClosers = [regex]::Matches($source, '}').Count
    $labeledClosers = 0
  }
  return [pscustomobject][ordered]@{
    source_lines = $nonEmptyLines.Count
    utf8_bytes = [Text.Encoding]::UTF8.GetByteCount($source)
    lexical_tokens = $tokens.Count
    punctuation_tokens = $punctuation.Count
    block_closers = $blockClosers
    labeled_block_closers = $labeledClosers
  }
}

function New-Aggregate([object[]]$Rows, [string]$Syntax, [string]$Cohort) {
  $selected = @($Rows | Where-Object { $_.syntax -eq $Syntax -and ($Cohort -eq 'all' -or $_.cohort -eq $Cohort) })
  $sumLines = [int](($selected | Measure-Object source_lines -Sum).Sum)
  $sumBytes = [int](($selected | Measure-Object utf8_bytes -Sum).Sum)
  $sumTokens = [int](($selected | Measure-Object lexical_tokens -Sum).Sum)
  $sumPunctuation = [int](($selected | Measure-Object punctuation_tokens -Sum).Sum)
  $sumClosers = [int](($selected | Measure-Object block_closers -Sum).Sum)
  $sumLabeledClosers = [int](($selected | Measure-Object labeled_block_closers -Sum).Sum)
  return [pscustomobject][ordered]@{
    syntax = $Syntax
    cohort = $Cohort
    files = $selected.Count
    source_lines = $sumLines
    utf8_bytes = $sumBytes
    lexical_tokens = $sumTokens
    punctuation_tokens = $sumPunctuation
    block_closers = $sumClosers
    labeled_block_closers = $sumLabeledClosers
    close_label_coverage = if ($sumClosers -eq 0) { 0 } else { [Math]::Round($sumLabeledClosers / $sumClosers, 4) }
    average_lines = [Math]::Round($sumLines / $selected.Count, 2)
    average_tokens = [Math]::Round($sumTokens / $selected.Count, 2)
  }
}

function Write-Utf8NoBom([string]$Path, [string]$Text) {
  $encoding = New-Object Text.UTF8Encoding($false)
  [IO.File]::WriteAllText($Path, $Text, $encoding)
}

$root = (Resolve-Path $RepositoryRoot).Path
$candidateRoots = [ordered]@{
  A0 = 'semantic-cases'
  B = 'syntax-candidates/b'
  C = 'syntax-candidates/c'
}
$rows = @()

foreach ($syntax in $candidateRoots.Keys) {
  $base = Join-Path $root $candidateRoots[$syntax]
  $files = @(Get-ChildItem -LiteralPath $base -Recurse -Filter '*.sico' | Sort-Object FullName)
  if ($files.Count -ne 54) {
    throw "expected 54 $syntax files, found $($files.Count)"
  }
  foreach ($file in $files) {
    $relative = $file.FullName.Substring($base.Length + 1).Replace('\', '/')
    $measurement = Measure-Source (Read-NormalizedText $file.FullName) $syntax
    $rows += [pscustomobject][ordered]@{
      syntax = $syntax
      cohort = Get-Cohort $relative
      path = $relative
      source_lines = $measurement.source_lines
      utf8_bytes = $measurement.utf8_bytes
      lexical_tokens = $measurement.lexical_tokens
      punctuation_tokens = $measurement.punctuation_tokens
      block_closers = $measurement.block_closers
      labeled_block_closers = $measurement.labeled_block_closers
    }
  }
}

$referencePaths = @($rows | Where-Object syntax -eq 'A0' | ForEach-Object path | Sort-Object)
foreach ($syntax in @('B', 'C')) {
  $paths = @($rows | Where-Object syntax -eq $syntax | ForEach-Object path | Sort-Object)
  if (($paths -join "`n") -cne ($referencePaths -join "`n")) {
    throw "$syntax paths do not mirror A0"
  }
}

$aggregates = @()
foreach ($cohort in @('round-1', 'round-2', 'all')) {
  foreach ($syntax in @('A0', 'B', 'C')) {
    $aggregates += New-Aggregate $rows $syntax $cohort
  }
}

$document = [pscustomobject][ordered]@{
  schema_version = 1
  method = 'sico-static-source-metrics-v1'
  metadata_removed = @('case', 'syntax', 'expect', 'semantics')
  cohorts = [pscustomobject][ordered]@{
    'round-1' = @('numbers-units', 'nominal-invariants', 'exhaustive-match', 'result-mapping')
    'round-2' = @('effects-capabilities', 'affine-resources', 'future-task', 'stream', 'component-call', 'revision')
  }
  aggregates = @($aggregates)
  files = @($rows | Sort-Object syntax, path)
}

$json = (($document | ConvertTo-Json -Depth 10).Replace("`r`n", "`n").Replace("`r", "`n")) + "`n"
if (-not [string]::IsNullOrWhiteSpace($CheckPath)) {
  $fullCheckPath = [IO.Path]::GetFullPath($CheckPath)
  if (-not (Test-Path -LiteralPath $fullCheckPath)) {
    throw "syntax metric snapshot does not exist: $fullCheckPath"
  }
  if ((Read-NormalizedText $fullCheckPath) -cne $json) {
    throw "syntax metric snapshot is stale: $fullCheckPath"
  }
}
if (-not [string]::IsNullOrWhiteSpace($OutputPath)) {
  $fullOutputPath = [IO.Path]::GetFullPath($OutputPath)
  $parent = Split-Path -Parent $fullOutputPath
  if (-not (Test-Path -LiteralPath $parent)) {
    New-Item -ItemType Directory -Path $parent -Force | Out-Null
  }
  Write-Utf8NoBom $fullOutputPath $json
}

$all = @($aggregates | Where-Object cohort -eq 'all')
$summary = ($all | ForEach-Object { "$($_.syntax)=$($_.files)/$($_.source_lines)/$($_.utf8_bytes)/$($_.lexical_tokens)/$($_.punctuation_tokens)/$($_.close_label_coverage)" }) -join ' '
Write-Output "SYNTAX_METRICS_OK metric=files/lines/bytes/tokens/punctuation/close-label-coverage $summary"
