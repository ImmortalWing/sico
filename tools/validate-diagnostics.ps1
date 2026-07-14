param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'

function Read-Json([string]$Path) {
  return Get-Content -LiteralPath $Path -Encoding UTF8 -Raw | ConvertFrom-Json
}

function Read-NormalizedText([string]$Path) {
  $text = [IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8)
  return $text.Replace("`r`n", "`n").Replace("`r", "`n")
}

function Read-Header([string]$Text, [string]$Name, [string]$Path) {
  $match = [regex]::Match($Text, "(?m)^// $([regex]::Escape($Name)): (.+)$")
  if (-not $match.Success) {
    throw "$Path is missing metadata '$Name'"
  }
  return $match.Groups[1].Value
}

function Get-Properties($Value) {
  return @($Value.PSObject.Properties | ForEach-Object { $_.Name })
}

function Require-Property($Value, [string]$Name, [string]$Context) {
  if ($null -eq $Value.PSObject.Properties[$Name]) {
    throw "$Context is missing $Name"
  }
  return $Value.$Name
}

function Compare-StringSets([string[]]$Actual, [string[]]$Expected, [string]$Context) {
  $actualSorted = @($Actual | Sort-Object -Unique)
  $expectedSorted = @($Expected | Sort-Object -Unique)
  if (($actualSorted -join "`n") -ne ($expectedSorted -join "`n")) {
    throw "$Context arguments differ: expected [$($expectedSorted -join ', ')], found [$($actualSorted -join ', ')]"
  }
}

function Render-Message([string]$Template, $Arguments, [string]$Context) {
  $rendered = $Template
  foreach ($property in @($Arguments.PSObject.Properties)) {
    $value = $property.Value
    if ($value -is [Array]) {
      $value = (@($value) -join ', ')
    } elseif ($value -isnot [string]) {
      throw "$Context argument '$($property.Name)' must be a string or string array"
    }
    if ([string]::IsNullOrEmpty([string]$value)) {
      throw "$Context argument '$($property.Name)' must not be empty"
    }
    $rendered = $rendered.Replace("{$($property.Name)}", [string]$value)
  }
  if ($rendered -match '\{[a-z][a-z0-9_]*\}') {
    throw "$Context left an unrendered message argument: $rendered"
  }
  return $rendered
}

function Assert-Position($Position, [string]$Context) {
  foreach ($name in @('byte', 'line', 'column')) {
    $value = Require-Property $Position $name $Context
    if ($value -isnot [int] -and $value -isnot [long]) {
      throw "$Context $name must be an integer"
    }
  }
  if ([int64]$Position.byte -lt 0 -or [int64]$Position.line -lt 1 -or [int64]$Position.column -lt 1) {
    throw "$Context contains an invalid coordinate"
  }
}

function Assert-Range($Range, [string]$Context) {
  $start = Require-Property $Range 'start' $Context
  $end = Require-Property $Range 'end' $Context
  Assert-Position $start "$Context start"
  Assert-Position $end "$Context end"
  if ([int64]$start.byte -gt [int64]$end.byte) {
    throw "$Context range start is after end"
  }
  if ([int64]$start.line -gt [int64]$end.line -or
      ([int64]$start.line -eq [int64]$end.line -and [int64]$start.column -gt [int64]$end.column)) {
    throw "$Context display range start is after end"
  }
}

function Assert-RelativeFile([string]$File, [string]$Context) {
  if ([string]::IsNullOrWhiteSpace($File) -or $File.Contains('\') -or
      [IO.Path]::IsPathRooted($File) -or $File -match '(^|/)\.\.(/|$)') {
    throw "$Context file must be a normalized workspace-relative path"
  }
}

function Get-SeverityRank([string]$Severity) {
  switch ($Severity) {
    'error' { return 0 }
    'warning' { return 1 }
    'info' { return 2 }
    default { throw "unknown severity: $Severity" }
  }
}

function Compare-Diagnostic($Left, $Right) {
  $comparison = [StringComparer]::Ordinal.Compare([string]$Left.file, [string]$Right.file)
  if ($comparison -ne 0) { return $comparison }
  $comparison = [int64]$Left.range.start.byte - [int64]$Right.range.start.byte
  if ($comparison -ne 0) { return [Math]::Sign($comparison) }
  $comparison = (Get-SeverityRank $Left.severity) - (Get-SeverityRank $Right.severity)
  if ($comparison -ne 0) { return [Math]::Sign($comparison) }
  $comparison = [StringComparer]::Ordinal.Compare([string]$Left.code, [string]$Right.code)
  if ($comparison -ne 0) { return $comparison }
  return [StringComparer]::Ordinal.Compare([string]$Left.id, [string]$Right.id)
}

function Assert-Envelope($Envelope, [hashtable]$CatalogByCode, [string]$Context) {
  if ((Require-Property $Envelope 'schema' $Context) -ne 'sico.diagnostics.v0') {
    throw "$Context has an unsupported schema"
  }
  if ((Require-Property $Envelope 'protocol_version' $Context) -ne 0) {
    throw "$Context has an unsupported protocol_version"
  }
  $tool = Require-Property $Envelope 'tool' $Context
  if ([string]::IsNullOrWhiteSpace([string](Require-Property $tool 'name' "$Context tool")) -or
      [string]::IsNullOrWhiteSpace([string](Require-Property $tool 'version' "$Context tool"))) {
    throw "$Context tool name/version must not be empty"
  }
  $coordinates = Require-Property $Envelope 'coordinate_system' $Context
  $expectedCoordinates = @{
    encoding = 'utf-8'
    byte_base = 0
    byte_end = 'exclusive'
    line_base = 1
    column_base = 1
    column_unit = 'unicode-scalar-value'
  }
  foreach ($entry in $expectedCoordinates.GetEnumerator()) {
    if ((Require-Property $coordinates $entry.Key "$Context coordinate_system") -ne $entry.Value) {
      throw "$Context coordinate_system.$($entry.Key) is invalid"
    }
  }

  $diagnostics = @(Require-Property $Envelope 'diagnostics' $Context)
  $seenIds = @{}
  $seenDiagnosticKeys = @{}
  $previous = $null
  $severityCounts = @{ error = 0; warning = 0; info = 0 }
  foreach ($diagnostic in $diagnostics) {
    $id = [string](Require-Property $diagnostic 'id' "$Context diagnostic")
    if ($id -notmatch '^d[1-9][0-9]*$' -or $seenIds.ContainsKey($id)) {
      throw "$Context diagnostic id is invalid or duplicate: $id"
    }
    $code = [string](Require-Property $diagnostic 'code' "$Context diagnostic $id")
    $key = [string](Require-Property $diagnostic 'key' "$Context diagnostic $id")
    if (-not $CatalogByCode.ContainsKey($code)) {
      throw "$Context diagnostic $id has unknown code: $code"
    }
    $definition = $CatalogByCode[$code]
    if ($key -ne $definition.key) {
      throw "$Context diagnostic $id key does not match $code"
    }
    $severity = [string](Require-Property $diagnostic 'severity' "$Context diagnostic $id")
    [void](Get-SeverityRank $severity)
    $severityCounts[$severity]++
    $kind = [string](Require-Property $diagnostic 'kind' "$Context diagnostic $id")
    if ($kind -notin @('root', 'cascade')) {
      throw "$Context diagnostic $id has invalid kind"
    }
    if ($kind -eq 'cascade') {
      $causedBy = [string](Require-Property $diagnostic 'caused_by' "$Context diagnostic $id cascade")
      if (-not $seenIds.ContainsKey($causedBy)) {
        throw "$Context diagnostic $id cascade caused_by must reference an earlier diagnostic"
      }
    } elseif ($null -ne $diagnostic.PSObject.Properties['caused_by']) {
      throw "$Context diagnostic $id root must not contain caused_by"
    }

    $message = [string](Require-Property $diagnostic 'message' "$Context diagnostic $id")
    if ([string]::IsNullOrEmpty($message) -or $message.Contains("`n") -or
        [Text.Encoding]::UTF8.GetByteCount($message) -gt 120) {
      throw "$Context diagnostic $id message must be one line and at most 120 UTF-8 bytes"
    }
    $arguments = Require-Property $diagnostic 'arguments' "$Context diagnostic $id"
    Compare-StringSets (Get-Properties $arguments) @($definition.required_arguments) "$Context diagnostic $id"
    $rendered = Render-Message $definition.message_template $arguments "$Context diagnostic $id"
    if ($message -ne $rendered) {
      throw "$Context diagnostic $id message does not match the catalog: $rendered"
    }
    if ($null -ne $diagnostic.PSObject.Properties['hint']) {
      $hint = [string]$diagnostic.hint
      if ([string]::IsNullOrEmpty($hint) -or $hint.Contains("`n") -or
          [Text.Encoding]::UTF8.GetByteCount($hint) -gt 100) {
        throw "$Context diagnostic $id hint must be one line and at most 100 UTF-8 bytes"
      }
    }

    $file = [string](Require-Property $diagnostic 'file' "$Context diagnostic $id")
    Assert-RelativeFile $file "$Context diagnostic $id"
    $range = Require-Property $diagnostic 'range' "$Context diagnostic $id"
    Assert-Range $range "$Context diagnostic $id"
    if ($null -ne $diagnostic.PSObject.Properties['related']) {
      foreach ($related in @($diagnostic.related)) {
        $relatedFile = [string](Require-Property $related 'file' "$Context diagnostic $id related")
        Assert-RelativeFile $relatedFile "$Context diagnostic $id related"
        Assert-Range (Require-Property $related 'range' "$Context diagnostic $id related") "$Context diagnostic $id related"
        $relatedMessage = [string](Require-Property $related 'message' "$Context diagnostic $id related")
        if ([string]::IsNullOrEmpty($relatedMessage) -or $relatedMessage.Contains("`n") -or
            [Text.Encoding]::UTF8.GetByteCount($relatedMessage) -gt 120) {
          throw "$Context diagnostic $id related message is invalid"
        }
      }
    }

    if ($null -ne $previous -and (Compare-Diagnostic $previous $diagnostic) -gt 0) {
      throw "$Context diagnostics are not in stable order at $id"
    }
    $argumentJson = $arguments | ConvertTo-Json -Compress -Depth 10
    $dedupKey = "$code`n$file`n$($range.start.byte)`n$($range.end.byte)`n$argumentJson"
    if ($seenDiagnosticKeys.ContainsKey($dedupKey)) {
      throw "$Context contains a duplicate diagnostic: $id"
    }
    $seenDiagnosticKeys[$dedupKey] = $true
    $seenIds[$id] = $true
    $previous = $diagnostic
  }

  $summary = Require-Property $Envelope 'summary' $Context
  foreach ($name in @('emitted', 'errors', 'warnings', 'info', 'suppressed', 'truncated')) {
    $value = Require-Property $summary $name "$Context summary"
    if (($value -isnot [int] -and $value -isnot [long]) -or [int64]$value -lt 0) {
      throw "$Context summary.$name must be a non-negative integer"
    }
  }
  if ([int]$summary.emitted -ne $diagnostics.Count -or
      [int]$summary.errors -ne $severityCounts.error -or
      [int]$summary.warnings -ne $severityCounts.warning -or
      [int]$summary.info -ne $severityCounts.info) {
    throw "$Context summary does not match emitted diagnostics"
  }
}

$root = (Resolve-Path $RepositoryRoot).Path
$diagnosticsRoot = Join-Path $root 'diagnostics'
$catalog = Read-Json (Join-Path $diagnosticsRoot 'catalog.json')
if ($catalog.schema -ne 'sico.diagnostic-catalog.v0' -or $catalog.catalog_version -ne 0 -or
    $catalog.message_locale -ne 'en-US') {
  throw 'unsupported diagnostic catalog metadata'
}

$partitions = @($catalog.partitions)
if ($partitions.Count -ne 9) {
  throw "expected 9 diagnostic partitions, found $($partitions.Count)"
}
$previousPartitionEnd = 999
foreach ($partition in $partitions) {
  if ([int]$partition.start -ne $previousPartitionEnd + 1 -or
      [int]$partition.end -ne [int]$partition.start + 999 -or
      $partition.status -notin @('active', 'reserved')) {
    throw "invalid diagnostic partition: $($partition.name)"
  }
  $previousPartitionEnd = [int]$partition.end
}

$catalogByCode = @{}
$catalogByKey = @{}
$previousCodeNumber = 0
foreach ($definition in @($catalog.diagnostics)) {
  $code = [string]$definition.code
  $key = [string]$definition.key
  if ($code -notmatch '^E([1-9][0-9]{3})$') {
    throw "invalid diagnostic code: $code"
  }
  $codeNumber = [int]$Matches[1]
  if ($codeNumber -le $previousCodeNumber) {
    throw "diagnostic catalog is not in ascending code order at $code"
  }
  $previousCodeNumber = $codeNumber
  if ($catalogByCode.ContainsKey($code) -or $catalogByKey.ContainsKey($key) -or
      $key -notmatch '^[A-Z][A-Z0-9_]*$') {
    throw "duplicate or invalid diagnostic identity: $code $key"
  }
  $partition = @($partitions | Where-Object {
    $codeNumber -ge [int]$_.start -and $codeNumber -le [int]$_.end
  })
  if ($partition.Count -ne 1 -or $partition[0].status -ne 'active') {
    throw "$code is not inside one active partition"
  }
  if ($definition.severity -ne 'error' -or [string]::IsNullOrWhiteSpace([string]$definition.category)) {
    throw "$code has invalid severity/category"
  }
  $template = [string]$definition.message_template
  if ([string]::IsNullOrEmpty($template) -or $template.Contains("`n") -or $template.EndsWith('.')) {
    throw "$code message template must be a non-empty line without a trailing period"
  }
  $placeholders = @([regex]::Matches($template, '\{([a-z][a-z0-9_]*)\}') | ForEach-Object {
    $_.Groups[1].Value
  })
  $required = @($definition.required_arguments)
  if (($placeholders -join "`n") -ne ($required -join "`n") -or
      @($required | Sort-Object -Unique).Count -ne $required.Count) {
    throw "$code message placeholders and required_arguments differ"
  }
  $catalogByCode[$code] = $definition
  $catalogByKey[$key] = $definition
}

$caseMap = Read-Json (Join-Path $diagnosticsRoot 'semantic-case-map.json')
if ($caseMap.schema -ne 'sico.semantic-diagnostic-cases.v0' -or $caseMap.catalog -ne 'catalog.json') {
  throw 'unsupported semantic diagnostic case map'
}
$cases = @($caseMap.cases)
$invalidSources = @(Get-ChildItem (Join-Path $root 'semantic-cases') -Recurse -Filter '*.sico' -File |
  Where-Object { $_.FullName -match '[\\/]invalid[\\/]' })
if ($cases.Count -ne $invalidSources.Count) {
  throw "expected one diagnostic case entry per invalid source: map=$($cases.Count) sources=$($invalidSources.Count)"
}

$seenCases = @{}
$seenSources = @{}
$usedKeys = @{}
$maxMessageBytes = 0
foreach ($caseEntry in $cases) {
  $caseId = [string]$caseEntry.case
  $source = [string]$caseEntry.source
  $code = [string]$caseEntry.code
  $key = [string]$caseEntry.key
  if ($caseId -notmatch '^[A-Z]+-[0-9]{3}$' -or $seenCases.ContainsKey($caseId)) {
    throw "duplicate or invalid diagnostic case id: $caseId"
  }
  if ($seenSources.ContainsKey($source) -or $source.Contains('\') -or
      $source -notmatch '^semantic-cases/[^/]+/invalid/[^/]+\.sico$') {
    throw "duplicate or invalid diagnostic case source: $source"
  }
  $sourcePath = Join-Path $root $source
  if (-not (Test-Path -LiteralPath $sourcePath)) {
    throw "missing diagnostic case source: $source"
  }
  $sourceText = Read-NormalizedText $sourcePath
  if ((Read-Header $sourceText 'case' $source) -ne $caseId -or
      (Read-Header $sourceText 'expect' $source) -ne "reject($key)") {
    throw "diagnostic case metadata mismatch: $caseId"
  }
  if (-not $catalogByCode.ContainsKey($code) -or $catalogByCode[$code].key -ne $key) {
    throw "diagnostic case identity mismatch: $caseId -> $code $key"
  }
  $definition = $catalogByCode[$code]
  Compare-StringSets (Get-Properties $caseEntry.arguments) @($definition.required_arguments) "case $caseId"
  $rendered = Render-Message $definition.message_template $caseEntry.arguments "case $caseId"
  if ($rendered -ne [string]$caseEntry.expected_message) {
    throw "case $caseId expected_message does not match catalog render: $rendered"
  }
  $messageBytes = [Text.Encoding]::UTF8.GetByteCount($rendered)
  if ($messageBytes -gt 120) {
    throw "case $caseId message exceeds 120 UTF-8 bytes"
  }
  $maxMessageBytes = [Math]::Max($maxMessageBytes, $messageBytes)

  $sourceParts = $source -split '/'
  $readmePath = Join-Path $root "semantic-cases/$($sourceParts[1])/README.md"
  $readmeText = Read-NormalizedText $readmePath
  $matchingRows = @($readmeText -split "`n" | Where-Object {
    $_.StartsWith('|') -and $_.Contains($caseId)
  })
  if ($matchingRows.Count -ne 1) {
    throw "case $caseId must have exactly one README row"
  }
  $columns = @($matchingRows[0].Split('|') | ForEach-Object { $_.Trim() })
  if ($columns.Count -lt 5 -or $columns[2].Trim('`') -ne $key -or $columns[3] -ne $rendered) {
    throw "case $caseId README key/message differs from the catalog"
  }

  $seenCases[$caseId] = $true
  $seenSources[$source] = $true
  $usedKeys[$key] = $true
}
Compare-StringSets @($usedKeys.Keys) @($catalogByKey.Keys) 'catalog usage'

$schema = Read-Json (Join-Path $diagnosticsRoot 'schema/diagnostics-v0.schema.json')
if ($schema.'$schema' -ne 'https://json-schema.org/draft/2020-12/schema' -or
    $schema.properties.schema.const -ne 'sico.diagnostics.v0' -or
    $schema.properties.protocol_version.const -ne 0) {
  throw 'diagnostics JSON Schema metadata is invalid'
}

$fixtureManifest = Read-Json (Join-Path $diagnosticsRoot 'fixtures/manifest.json')
if ($fixtureManifest.schema -ne 'sico.diagnostic-fixtures.v0') {
  throw 'unsupported diagnostic fixture manifest'
}
$acceptedFixtures = 0
$rejectedFixtures = 0
foreach ($fixture in @($fixtureManifest.fixtures)) {
  $fixturePath = Join-Path $diagnosticsRoot "fixtures/$($fixture.path)"
  if (-not (Test-Path -LiteralPath $fixturePath)) {
    throw "missing diagnostic fixture: $($fixture.path)"
  }
  $fixtureEnvelope = Read-Json $fixturePath
  $caught = $null
  try {
    Assert-Envelope $fixtureEnvelope $catalogByCode "fixture $($fixture.path)"
  } catch {
    $caught = "$($_.Exception.Message) [$($_.ScriptStackTrace)]"
  }
  if ($fixture.expect -eq 'accept') {
    if ($null -ne $caught) {
      throw "fixture $($fixture.path) should pass: $caught"
    }
    $acceptedFixtures++
  } elseif ($fixture.expect -eq 'reject') {
    if ($null -eq $caught) {
      throw "fixture $($fixture.path) should be rejected"
    }
    if ([string]::IsNullOrWhiteSpace([string]$fixture.error) -or -not $caught.Contains([string]$fixture.error)) {
      throw "fixture $($fixture.path) failed for the wrong reason: $caught"
    }
    $rejectedFixtures++
  } else {
    throw "fixture $($fixture.path) has invalid expectation"
  }
}

Write-Output "DIAGNOSTICS_OK catalog=$($catalogByCode.Count) cases=$($cases.Count) partitions=$($partitions.Count) fixtures=$(@($fixtureManifest.fixtures).Count) accepted=$acceptedFixtures rejected=$rejectedFixtures max_message_bytes=$maxMessageBytes"
