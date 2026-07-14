Set-StrictMode -Version Latest

function Read-Utf8Json([string]$Path) {
  return Get-Content -LiteralPath $Path -Encoding UTF8 -Raw | ConvertFrom-Json
}

function Read-NormalizedText([string]$Path) {
  $text = [IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8)
  return $text.Replace("`r`n", "`n").Replace("`r", "`n")
}

function Remove-SicoMetadata([string]$Text) {
  $lines = $Text -split "`n"
  $kept = foreach ($line in $lines) {
    if ($line -notmatch '^// (case|syntax|expect|semantics|mutation|source-case|recovery):') {
      $line
    }
  }
  return $kept -join "`n"
}

function Read-SicoMetadata([string]$Text, [string]$Name) {
  $match = [regex]::Match($Text, "(?m)^// $([regex]::Escape($Name)): (.+)$")
  if (-not $match.Success) {
    throw "Sico source is missing metadata '$Name'"
  }
  return $match.Groups[1].Value
}

function Normalize-SicoSource([string]$Text) {
  $normalized = $Text.Replace("`r`n", "`n").Replace("`r", "`n")
  $normalized = Remove-SicoMetadata $normalized
  return $normalized.Trim("`n") + "`n"
}

function Normalize-SubmittedSicoSource([string]$Text) {
  $normalized = $Text.Replace("`r`n", "`n").Replace("`r", "`n")
  return $normalized.Trim("`n") + "`n"
}

function Write-Utf8NoBom([string]$Path, [string]$Text) {
  $encoding = New-Object Text.UTF8Encoding($false)
  [IO.File]::WriteAllText($Path, $Text, $encoding)
}

function Get-ObjectProperty([object]$Object, [string]$Name, [string]$Context) {
  $property = $Object.PSObject.Properties[$Name]
  if ($null -eq $property) {
    throw "$Context is missing property '$Name'"
  }
  return $property.Value
}

function Get-AiEvalTasks([string]$RepositoryRoot) {
  $root = (Resolve-Path $RepositoryRoot).Path
  $generation = Read-Utf8Json (Join-Path $root 'ai-eval/tasks/generation.json')
  $understanding = Read-Utf8Json (Join-Path $root 'ai-eval/tasks/understanding.json')
  $repair = Read-Utf8Json (Join-Path $root 'ai-eval/tasks/repair.json')
  if ($generation.schema_version -ne 1 -or $generation.category -ne 'generation') {
    throw 'invalid generation task manifest header'
  }
  if ($understanding.schema_version -ne 1 -or $understanding.category -ne 'understanding') {
    throw 'invalid understanding task manifest header'
  }
  if ($repair.schema_version -ne 1 -or $repair.category -ne 'repair') {
    throw 'invalid repair task manifest header'
  }
  $tasks = @()

  foreach ($entry in @($generation.tasks)) {
    if ([string]$entry.pair_id -notmatch '^GEN-.+$') {
      throw "invalid generation pair_id: $($entry.pair_id)"
    }
    foreach ($syntax in @('A0', 'B', 'C')) {
      $suffix = $entry.pair_id.Substring(4)
      $tasks += [pscustomobject][ordered]@{
        task_id = "GEN-$syntax-$suffix"
        pair_id = [string]$entry.pair_id
        case_id = [string]$entry.case_id
        category = 'generation'
        syntax = $syntax
        guide_path = "ai-eval/guides/$($syntax.ToLowerInvariant()).md"
        prompt = [string]$entry.prompt
        input_path = $null
        expected_path = [string]$entry.expected.$syntax
        expected_answer = $null
        diagnostic = $null
      }
    }
  }

  foreach ($entry in @($understanding.tasks)) {
    if ([string]$entry.pair_id -notmatch '^UNDERSTAND-.+$') {
      throw "invalid understanding pair_id: $($entry.pair_id)"
    }
    foreach ($syntax in @('A0', 'B', 'C')) {
      $suffix = $entry.pair_id.Substring(11)
      $tasks += [pscustomobject][ordered]@{
        task_id = "UNDERSTAND-$syntax-$suffix"
        pair_id = [string]$entry.pair_id
        case_id = [string]$entry.case_id
        category = 'understanding'
        syntax = $syntax
        guide_path = "ai-eval/guides/$($syntax.ToLowerInvariant()).md"
        prompt = [string]$entry.prompt
        input_path = [string]$entry.source.$syntax
        expected_path = $null
        expected_answer = $entry.expected
        diagnostic = $null
      }
    }
  }

  foreach ($entry in @($repair.tasks)) {
    $tasks += [pscustomobject][ordered]@{
      task_id = [string]$entry.task_id
      pair_id = "REPAIR-$($entry.mutation)"
      case_id = [string]$entry.mutation
      category = 'repair'
      syntax = [string]$entry.syntax
      guide_path = "ai-eval/guides/$(([string]$entry.syntax).ToLowerInvariant()).md"
      prompt = [string]$repair.prompt
      input_path = [string]$entry.input
      expected_path = [string]$entry.expected
      expected_answer = $null
      diagnostic = [string]$entry.diagnostic
    }
  }

  return @($tasks | Sort-Object task_id)
}

function New-AiEvalUserPrompt([string]$RepositoryRoot, [object]$Task) {
  $root = (Resolve-Path $RepositoryRoot).Path
  $guide = (Read-NormalizedText (Join-Path $root $Task.guide_path)).Trim()
  $parts = @(
    "Syntax candidate: $($Task.syntax)",
    "",
    "Syntax guide:",
    $guide,
    "",
    "Task:",
    $Task.prompt
  )

  if ($Task.category -eq 'understanding') {
    $source = Normalize-SicoSource (Read-NormalizedText (Join-Path $root $Task.input_path))
    $parts += @("", "Source:", $source.TrimEnd("`n"), "", "Output contract:", "Return only the requested JSON object.")
  }
  elseif ($Task.category -eq 'repair') {
    $source = Normalize-SicoSource (Read-NormalizedText (Join-Path $root $Task.input_path))
    $parts += @("", "Reported diagnostic: $($Task.diagnostic)", "", "Source with one injected error:", $source.TrimEnd("`n"), "", "Output contract:", "Return only the complete repaired Sico source.")
  }
  else {
    $parts += @("", "Output contract:", "Return only the complete Sico source.")
  }

  return ($parts -join "`n") + "`n"
}

function ConvertTo-CanonicalValue([object]$Value) {
  if ($null -eq $Value) {
    return 'null'
  }

  if ($Value -is [string] -or $Value -is [char] -or $Value -is [bool] -or
      $Value -is [byte] -or $Value -is [int16] -or $Value -is [int32] -or
      $Value -is [int64] -or $Value -is [decimal] -or $Value -is [double] -or
      $Value -is [single]) {
    return ConvertTo-Json -InputObject $Value -Compress
  }

  if ($Value -is [Collections.IDictionary]) {
    $parts = foreach ($key in @($Value.Keys | Sort-Object)) {
      (ConvertTo-Json -InputObject ([string]$key) -Compress) + ':' + (ConvertTo-CanonicalValue $Value[$key])
    }
    return '{' + ($parts -join ',') + '}'
  }

  if ($Value -is [Collections.IEnumerable]) {
    $items = @($Value | ForEach-Object { ConvertTo-CanonicalValue $_ } | Sort-Object)
    return '[' + ($items -join ',') + ']'
  }

  $objectParts = foreach ($property in @($Value.PSObject.Properties | Sort-Object Name)) {
    (ConvertTo-Json -InputObject $property.Name -Compress) + ':' + (ConvertTo-CanonicalValue $property.Value)
  }
  return '{' + ($objectParts -join ',') + '}'
}

function Get-Sha256([string]$Path) {
  return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash.ToLowerInvariant()
}
