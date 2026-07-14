param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'

function Read-Json([string]$Path) {
  return Get-Content -LiteralPath $Path -Encoding UTF8 -Raw | ConvertFrom-Json
}

function Require-Property($Value, [string]$Name, [string]$Context) {
  if ($null -eq $Value -or $null -eq $Value.PSObject.Properties[$Name]) {
    throw "$Context is missing $Name"
  }
  return $Value.$Name
}

function Get-PropertyNames($Value) {
  return @($Value.PSObject.Properties | ForEach-Object { $_.Name })
}

function ConvertTo-CanonicalJson($Value) {
  if ($null -eq $Value) { return 'null' }
  if ($Value -is [bool]) {
    if ($Value) { return 'true' }
    return 'false'
  }
  if ($Value -is [string] -or $Value -is [char]) {
    return ([string]$Value | ConvertTo-Json -Compress)
  }
  if ($Value -is [byte] -or $Value -is [sbyte] -or $Value -is [int16] -or
      $Value -is [uint16] -or $Value -is [int32] -or $Value -is [uint32] -or
      $Value -is [int64] -or $Value -is [uint64] -or $Value -is [single] -or
      $Value -is [double] -or $Value -is [decimal]) {
    return ($Value | ConvertTo-Json -Compress)
  }
  if ($Value -is [Array]) {
    $items = @($Value | ForEach-Object { ConvertTo-CanonicalJson $_ })
    return '[' + ($items -join ',') + ']'
  }
  $properties = @($Value.PSObject.Properties)
  if ($Value -is [pscustomobject] -or $Value -is [hashtable]) {
    $names = [string[]]@($properties | ForEach-Object { $_.Name })
    [Array]::Sort($names, [StringComparer]::Ordinal)
    $members = @($names | ForEach-Object {
      (ConvertTo-CanonicalJson $_) + ':' + (ConvertTo-CanonicalJson $Value.$_)
    })
    return '{' + ($members -join ',') + '}'
  }
  throw "cannot canonicalize JSON value of type $($Value.GetType().FullName)"
}

function Assert-RelativeFile([string]$File, [string]$Context) {
  if ([string]::IsNullOrWhiteSpace($File) -or $File.Contains('\') -or
      [IO.Path]::IsPathRooted($File) -or $File -match '(^|/)\.\.(/|$)') {
    throw "$Context file must be a normalized workspace-relative path"
  }
}

function Assert-SemanticId([string]$Id, [regex]$Pattern, [string]$Context) {
  if (-not $Pattern.IsMatch($Id) -or $Id -match '(byte|line|column|offset)=[0-9]') {
    throw "$Context has an invalid semantic ID: $Id"
  }
}

function Assert-Quality($Quality, [string]$Context) {
  $state = [string](Require-Property $Quality 'state' $Context)
  $reasons = @(Require-Property $Quality 'reasons' $Context)
  $blockedBy = @(Require-Property $Quality 'blocked_by' $Context)
  if ($state -notin @('complete', 'partial', 'unknown')) {
    throw "$Context has invalid completeness: $state"
  }
  foreach ($reason in $reasons) {
    if ([string]::IsNullOrWhiteSpace([string]$reason)) {
      throw "$Context contains an empty completeness reason"
    }
  }
  foreach ($code in $blockedBy) {
    if ([string]$code -notmatch '^E[1-9][0-9]{3}$') {
      throw "$Context has invalid blocking diagnostic: $code"
    }
  }
  if ($state -eq 'complete' -and ($reasons.Count -ne 0 -or $blockedBy.Count -ne 0)) {
    throw "$Context complete value cannot have reasons or blocking diagnostics"
  }
  if ($state -ne 'complete' -and $reasons.Count -eq 0 -and $blockedBy.Count -eq 0) {
    throw "$Context incomplete value must explain why"
  }
}

function Get-ScalarCount([string]$Text) {
  $count = 0
  for ($index = 0; $index -lt $Text.Length; $index++) {
    if ([char]::IsHighSurrogate($Text[$index]) -and $index + 1 -lt $Text.Length -and
        [char]::IsLowSurrogate($Text[$index + 1])) {
      $index++
    }
    $count++
  }
  return $count
}

function Get-PositionAtByte([byte[]]$Bytes, [int64]$Offset, [string]$Context) {
  if ($Offset -lt 0 -or $Offset -gt $Bytes.Length) {
    throw "$Context byte offset is outside the file"
  }
  $strictUtf8 = New-Object Text.UTF8Encoding($false, $true)
  try {
    $prefix = $strictUtf8.GetString($Bytes, 0, [int]$Offset)
  } catch {
    throw "$Context byte offset is not on a UTF-8 boundary"
  }
  $line = 1
  foreach ($character in $prefix.ToCharArray()) {
    if ($character -eq "`n") { $line++ }
  }
  $lastNewline = $prefix.LastIndexOf("`n")
  $lineText = $prefix.Substring($lastNewline + 1)
  return @{ line = $line; column = (Get-ScalarCount $lineText) + 1 }
}

function Assert-Source($Source, [string]$Root, [string]$Context, [string]$ExpectedText = '') {
  $file = [string](Require-Property $Source 'file' $Context)
  Assert-RelativeFile $file $Context
  $path = Join-Path $Root $file
  if (-not (Test-Path -LiteralPath $path)) {
    throw "$Context source file does not exist: $file"
  }
  $range = Require-Property $Source 'range' $Context
  $start = Require-Property $range 'start' "$Context range"
  $end = Require-Property $range 'end' "$Context range"
  foreach ($positionName in @('start', 'end')) {
    $position = if ($positionName -eq 'start') { $start } else { $end }
    foreach ($name in @('byte', 'line', 'column')) {
      $value = Require-Property $position $name "$Context $positionName"
      if (($value -isnot [int] -and $value -isnot [long]) -or [int64]$value -lt 0) {
        throw "$Context $positionName.$name must be a non-negative integer"
      }
    }
    if ([int64]$position.line -lt 1 -or [int64]$position.column -lt 1) {
      throw "$Context $positionName line/column must be one-based"
    }
  }
  if ([int64]$start.byte -gt [int64]$end.byte) {
    throw "$Context range start is after end"
  }
  $bytes = [IO.File]::ReadAllBytes($path)
  $actualStart = Get-PositionAtByte $bytes ([int64]$start.byte) "$Context start"
  $actualEnd = Get-PositionAtByte $bytes ([int64]$end.byte) "$Context end"
  if ([int64]$start.line -ne $actualStart.line -or [int64]$start.column -ne $actualStart.column -or
      [int64]$end.line -ne $actualEnd.line -or [int64]$end.column -ne $actualEnd.column) {
    throw "$Context line/column does not match UTF-8 byte range"
  }
  if (-not [string]::IsNullOrEmpty($ExpectedText)) {
    $length = [int64]$end.byte - [int64]$start.byte
    $strictUtf8 = New-Object Text.UTF8Encoding($false, $true)
    $actualText = $strictUtf8.GetString($bytes, [int]$start.byte, [int]$length)
    if ($actualText -ne $ExpectedText) {
      throw "$Context range does not select '$ExpectedText': '$actualText'"
    }
  }
}

function Assert-Evidence($Evidence, [string]$Context) {
  $kind = [string](Require-Property $Evidence 'kind' $Context)
  $ref = [string](Require-Property $Evidence 'ref' $Context)
  if ($kind -notin @('source', 'analysis', 'semantic-rule', 'diagnostic', 'user-change') -or
      [string]::IsNullOrWhiteSpace($ref)) {
    throw "$Context has invalid evidence"
  }
  if ($kind -eq 'semantic-rule' -and $ref -notmatch '^SEM-[0-9]{3}$') {
    throw "$Context semantic-rule evidence must use a SEM id"
  }
  if ($kind -eq 'diagnostic' -and $ref -notmatch '^E[1-9][0-9]{3}$') {
    throw "$Context diagnostic evidence must use an error code"
  }
}

function Assert-Fact($Fact, [string]$Context) {
  $basis = [string](Require-Property $Fact 'basis' $Context)
  [void](Require-Property $Fact 'value' $Context)
  $evidence = @(Require-Property $Fact 'evidence' $Context)
  if ($basis -notin @('declared', 'verified', 'inferred') -or $evidence.Count -eq 0) {
    throw "$Context has invalid basis or empty evidence"
  }
  foreach ($item in $evidence) {
    Assert-Evidence $item "$Context evidence"
  }
  $evidenceKinds = @($evidence | ForEach-Object { [string]$_.kind })
  if ($basis -eq 'declared' -and 'source' -notin $evidenceKinds) {
    throw "$Context declared fact lacks source evidence"
  }
  if ($basis -eq 'verified') {
    $hasCompilerEvidence = $false
    foreach ($item in $evidence) {
      if ($item.kind -eq 'semantic-rule' -or
          ($item.kind -eq 'analysis' -and [string]$item.ref -match '^compiler:')) {
        $hasCompilerEvidence = $true
      }
    }
    if (-not $hasCompilerEvidence) {
      throw "$Context verified fact lacks compiler evidence"
    }
  }
}

function Assert-Facets($Facets, [string]$Context) {
  foreach ($property in @($Facets.PSObject.Properties)) {
    Assert-Fact $property.Value "$Context facet '$($property.Name)'"
  }
}

function Assert-BasisEvidence($Value, [string]$Context) {
  $fact = [pscustomobject]@{
    basis = Require-Property $Value 'basis' $Context
    value = $true
    evidence = Require-Property $Value 'evidence' $Context
  }
  Assert-Fact $fact $Context
}

function Assert-Request($Request, [hashtable]$KnownIds, [string[]]$Operations, $Protocol, [string]$Context) {
  if ((Require-Property $Request 'schema' $Context) -ne 'sico.semantic-query.v0' -or
      (Require-Property $Request 'protocol_version' $Context) -ne 0) {
    throw "$Context has unsupported request metadata"
  }
  $requestId = [string](Require-Property $Request 'request_id' $Context)
  if ($requestId -notmatch '^[A-Za-z0-9._-]+$') {
    throw "$Context has invalid request_id"
  }
  $operation = [string](Require-Property $Request 'operation' $Context)
  if ($operation -notin $Operations) {
    throw "$Context has unknown operation: $operation"
  }
  $targetId = [string](Require-Property (Require-Property $Request 'target' $Context) 'id' "$Context target")
  if (-not $KnownIds.ContainsKey($targetId)) {
    throw "$Context target is not in the snapshot: $targetId"
  }
  $options = Require-Property $Request 'options' $Context
  foreach ($name in @('max_depth', 'max_items', 'max_bytes')) {
    $value = Require-Property $options $name "$Context options"
    if ($value -isnot [int] -and $value -isnot [long]) {
      throw "$Context options.$name must be an integer"
    }
  }
  if ([int]$options.max_depth -lt 0 -or [int]$options.max_depth -gt [int]$Protocol.budget.limits.max_depth -or
      [int]$options.max_items -lt 1 -or [int]$options.max_items -gt [int]$Protocol.budget.limits.max_items -or
      [int]$options.max_bytes -lt 1024 -or [int]$options.max_bytes -gt [int]$Protocol.budget.limits.max_bytes) {
    throw "$Context options exceed protocol limits"
  }
  foreach ($name in @('include_private', 'include_source', 'transitive')) {
    if ((Require-Property $options $name "$Context options") -isnot [bool]) {
      throw "$Context options.$name must be boolean"
    }
  }
  if ($operation -eq 'impact') {
    $parameters = Require-Property $Request 'parameters' $Context
    $change = Require-Property $parameters 'change' "$Context parameters"
    if ([string]$change.kind -notin @('signature', 'behavior', 'type', 'error', 'effect', 'capability', 'contract', 'remove', 'rename') -or
        [string]::IsNullOrWhiteSpace([string]$change.summary)) {
      throw "$Context impact query requires a valid change"
    }
  }
  if ($operation -eq 'flow') {
    $parameters = Require-Property $Request 'parameters' $Context
    if ([string](Require-Property $parameters 'flow_kind' "$Context parameters") -notin
        @('state', 'data', 'error', 'effect', 'resource', 'task', 'revision')) {
      throw "$Context flow query requires a valid flow_kind"
    }
  }
}

function Assert-Response($Request, $Response, [hashtable]$KnownIds, [hashtable]$SymbolsById, [string]$Root, [string]$Context) {
  if ((Require-Property $Response 'schema' $Context) -ne 'sico.semantic-response.v0' -or
      (Require-Property $Response 'protocol_version' $Context) -ne 0) {
    throw "$Context has unsupported response metadata"
  }
  if ([string](Require-Property $Response 'request_id' $Context) -ne [string]$Request.request_id) {
    throw "$Context request_id mismatch"
  }
  if ([string](Require-Property $Response 'snapshot_id' $Context) -ne [string]$Request.snapshot_id) {
    throw "$Context snapshot mismatch"
  }
  if ([string](Require-Property $Response 'operation' $Context) -ne [string]$Request.operation) {
    throw "$Context operation mismatch"
  }
  $budget = Require-Property $Response 'budget' $Context
  foreach ($name in @('max_depth', 'max_items', 'max_bytes')) {
    if ([int](Require-Property $budget $name "$Context budget") -ne [int]$Request.options.$name) {
      throw "$Context budget.$name differs from request"
    }
  }
  foreach ($name in @('used_depth', 'used_items', 'used_bytes', 'omitted_items')) {
    $value = Require-Property $budget $name "$Context budget"
    if (($value -isnot [int] -and $value -isnot [long]) -or [int64]$value -lt 0) {
      throw "$Context budget.$name must be a non-negative integer"
    }
  }
  $truncated = Require-Property $budget 'truncated' "$Context budget"
  if ($truncated -isnot [bool]) {
    throw "$Context budget.truncated must be boolean"
  }
  if ([int]$budget.used_depth -gt [int]$budget.max_depth -or
      [int]$budget.used_items -gt [int]$budget.max_items -or
      [int]$budget.used_bytes -gt [int]$budget.max_bytes) {
    throw "$Context exceeded its declared budget"
  }

  $result = Require-Property $Response 'result' $Context
  $target = [string](Require-Property $result 'target' "$Context result")
  if ($target -ne [string]$Request.target.id -or -not $KnownIds.ContainsKey($target)) {
    throw "$Context result target mismatch"
  }
  $completeness = [string](Require-Property $result 'completeness' "$Context result")
  $reasons = @(Require-Property $result 'reasons' "$Context result")
  $blockedBy = @(Require-Property $result 'blocked_by' "$Context result")
  if ($completeness -notin @('complete', 'partial', 'unknown')) {
    throw "$Context result has invalid completeness"
  }
  if ($completeness -eq 'complete' -and ($reasons.Count -ne 0 -or $blockedBy.Count -ne 0)) {
    throw "$Context complete result cannot have reasons or blocking diagnostics"
  }
  if ($completeness -ne 'complete' -and $reasons.Count -eq 0 -and $blockedBy.Count -eq 0) {
    throw "$Context partial/unknown result must explain why"
  }
  if ([bool]$budget.truncated -or [int]$budget.omitted_items -gt 0) {
    if ($completeness -eq 'complete') {
      throw "$Context truncated result cannot be complete"
    }
    if (-not [bool]$budget.truncated -or [int]$budget.omitted_items -eq 0) {
      throw "$Context truncation flags are inconsistent"
    }
  } elseif ([int]$budget.omitted_items -ne 0) {
    throw "$Context non-truncated result cannot omit items"
  }

  if ($null -ne $result.PSObject.Properties['summary']) {
    Assert-Fact $result.summary "$Context result summary"
  }
  $items = @(Require-Property $result 'items' "$Context result")
  $edges = @(Require-Property $result 'edges' "$Context result")
  $nextQueries = @(Require-Property $result 'next_queries' "$Context result")
  if ([int]$budget.used_items -ne $items.Count) {
    throw "$Context used_items does not match result items"
  }
  $canonicalResult = ConvertTo-CanonicalJson $result
  $actualBytes = [Text.Encoding]::UTF8.GetByteCount($canonicalResult)
  if ([int]$budget.used_bytes -ne $actualBytes) {
    throw "$Context used_bytes mismatch: expected $actualBytes"
  }

  $seenItems = @{}
  foreach ($item in $items) {
    $id = [string](Require-Property $item 'id' "$Context item")
    if (-not $KnownIds.ContainsKey($id) -or $seenItems.ContainsKey($id)) {
      throw "$Context item id is unknown or duplicate: $id"
    }
    $seenItems[$id] = $true
    foreach ($name in @('kind', 'name', 'role', 'completeness')) {
      if ([string]::IsNullOrWhiteSpace([string](Require-Property $item $name "$Context item $id"))) {
        throw "$Context item $id has empty $name"
      }
    }
    if ($item.completeness -notin @('complete', 'partial', 'unknown')) {
      throw "$Context item $id has invalid completeness"
    }
    Assert-Facets (Require-Property $item 'facets' "$Context item $id") "$Context item $id"
    if ($null -ne $item.PSObject.Properties['reason']) {
      Assert-Fact $item.reason "$Context item $id reason"
    }
    if ($null -ne $item.PSObject.Properties['source']) {
      Assert-Source $item.source $Root "$Context item $id source"
    }
    if (-not [bool]$Request.options.include_private -and $SymbolsById.ContainsKey($id) -and
        $SymbolsById[$id].visibility -eq 'private') {
      throw "$Context returned a private symbol without permission: $id"
    }
    if (-not [bool]$Request.options.include_source -and
        $null -ne $item.PSObject.Properties['source_text']) {
      throw "$Context returned source text without permission"
    }
  }

  foreach ($edge in $edges) {
    $from = [string](Require-Property $edge 'from' "$Context edge")
    $to = [string](Require-Property $edge 'to' "$Context edge")
    if (-not $KnownIds.ContainsKey($from) -or -not $KnownIds.ContainsKey($to)) {
      throw "$Context edge endpoint is not in the snapshot: $from -> $to"
    }
    if ([string]::IsNullOrWhiteSpace([string](Require-Property $edge 'kind' "$Context edge"))) {
      throw "$Context edge kind is empty"
    }
    Assert-BasisEvidence $edge "$Context edge $from -> $to"
  }
  foreach ($nextQuery in $nextQueries) {
    if ([string]$nextQuery.operation -notin @('outline', 'describe', 'slice', 'impact', 'flow') -or
        -not $KnownIds.ContainsKey([string]$nextQuery.target)) {
      throw "$Context has an invalid next query"
    }
  }

  switch ([string]$Request.operation) {
    'outline' {
      if ($items.Count -eq 0 -or @($items | Where-Object role -ne 'module').Count -ne 0) {
        throw "$Context outline must return module items"
      }
    }
    'describe' {
      if ($items.Count -ne 1 -or $items[0].id -ne $target -or $items[0].role -ne 'target') {
        throw "$Context describe must return exactly one target item"
      }
    }
    'slice' {
      if (@($items | Where-Object id -eq $target).Count -ne 1) {
        throw "$Context slice must include its target"
      }
      foreach ($item in @($items | Where-Object id -ne $target)) {
        if ($null -eq $item.PSObject.Properties['reason']) {
          throw "$Context slice dependency lacks an inclusion reason"
        }
      }
    }
    'impact' {
      foreach ($item in $items) {
        if ($null -eq $item.PSObject.Properties['reason'] -or
            $null -eq $item.PSObject.Properties['via'] -or @($item.via).Count -eq 0 -or
            [string]$item.via[0] -ne $target) {
          throw "$Context impact item lacks a reason or target-rooted path"
        }
      }
    }
    'flow' {
      if ($edges.Count -eq 0) {
        throw "$Context flow must return at least one edge"
      }
      if ([string]$Request.parameters.flow_kind -eq 'state' -and $completeness -eq 'complete') {
        $remainingEdges = @($edges | Where-Object {
          $null -ne $_.PSObject.Properties['covers_remaining'] -and [bool]$_.covers_remaining
        })
        if ($remainingEdges.Count -ne 1) {
          throw "$Context complete state flow must represent remaining rejected combinations"
        }
      }
    }
  }
}

$root = (Resolve-Path $RepositoryRoot).Path
$semanticRoot = Join-Path $root 'semantic-index'
$protocol = Read-Json (Join-Path $semanticRoot 'protocol.json')
if ($protocol.schema -ne 'sico.semantic-query-protocol.v0' -or $protocol.protocol_version -ne 0 -or
    $protocol.budget.hard_unit -ne 'canonical-result-utf8-bytes') {
  throw 'unsupported semantic query protocol metadata'
}
$operations = @($protocol.operations)
if (($operations -join ',') -ne 'outline,describe,slice,impact,flow') {
  throw 'semantic query operations are missing or out of canonical order'
}
$semanticIdPattern = New-Object regex([string]$protocol.semantic_id.pattern, [Text.RegularExpressions.RegexOptions]::CultureInvariant)

foreach ($schemaFile in @('index-v0.schema.json', 'query-v0.schema.json', 'response-v0.schema.json')) {
  $schema = Read-Json (Join-Path $semanticRoot "schema/$schemaFile")
  if ($schema.'$schema' -ne 'https://json-schema.org/draft/2020-12/schema' -or
      [string]::IsNullOrWhiteSpace([string]$schema.'$id')) {
    throw "invalid JSON Schema metadata: $schemaFile"
  }
}

$index = Read-Json (Join-Path $semanticRoot 'fixtures/index.json')
if ($index.schema -ne 'sico.semantic-index.v0' -or $index.protocol_version -ne 0) {
  throw 'unsupported semantic index fixture metadata'
}
if ($index.producer.mode -eq 'fixture' -and [string]$index.snapshot.id -notmatch '^fixture:' -or
    $index.producer.mode -eq 'compiler' -and [string]$index.snapshot.id -notmatch '^sha256:[a-f0-9]{64}$') {
  throw 'snapshot id does not match producer mode'
}
Assert-Quality $index.snapshot.quality 'index snapshot'
Assert-SemanticId ([string]$index.package.id) $semanticIdPattern 'package'

$knownIds = @{}
$moduleIds = @{}
foreach ($module in @($index.modules)) {
  $id = [string]$module.id
  Assert-SemanticId $id $semanticIdPattern 'module'
  if ($knownIds.ContainsKey($id)) { throw "duplicate semantic ID: $id" }
  $sourceFile = [string](Require-Property $module 'source_file' "module $id")
  Assert-RelativeFile $sourceFile "module $id"
  if (-not (Test-Path -LiteralPath (Join-Path $root $sourceFile))) {
    throw "module source does not exist: $sourceFile"
  }
  Assert-Quality $module.quality "module $id"
  Assert-Facets $module.facets "module $id"
  $knownIds[$id] = $module
  $moduleIds[$id] = $true
}

$symbolsById = @{}
foreach ($symbol in @($index.symbols)) {
  $id = [string]$symbol.id
  Assert-SemanticId $id $semanticIdPattern 'symbol'
  if ($knownIds.ContainsKey($id)) { throw "duplicate semantic ID: $id" }
  if (-not $moduleIds.ContainsKey([string]$symbol.module_id)) {
    throw "symbol has unknown module: $id"
  }
  foreach ($name in @('kind', 'name', 'visibility')) {
    if ([string]::IsNullOrWhiteSpace([string](Require-Property $symbol $name "symbol $id"))) {
      throw "symbol $id has empty $name"
    }
  }
  if ($symbol.visibility -notin @('public', 'package', 'private')) {
    throw "symbol $id has invalid visibility"
  }
  Assert-Source $symbol.source $root "symbol $id" ([string]$symbol.name)
  Assert-Quality $symbol.quality "symbol $id"
  Assert-Facets $symbol.facets "symbol $id"
  $knownIds[$id] = $symbol
  $symbolsById[$id] = $symbol
}
$knownIds[[string]$index.package.id] = $index.package

foreach ($relation in @($index.relations)) {
  $from = [string]$relation.from
  $to = [string]$relation.to
  if (-not $knownIds.ContainsKey($from) -or -not $knownIds.ContainsKey($to)) {
    throw "index relation has unknown endpoint: $from -> $to"
  }
  if ([string]::IsNullOrWhiteSpace([string]$relation.kind)) {
    throw "index relation has an empty kind"
  }
  Assert-BasisEvidence $relation "index relation $from -> $to"
}

$sampleMatrix = Read-Json (Join-Path $semanticRoot 'sample-matrix.json')
if ($sampleMatrix.schema -ne 'sico.semantic-query-samples.v0') {
  throw 'unsupported semantic query sample matrix'
}
$samples = @($sampleMatrix.samples)
if ($samples.Count -ne 10) {
  throw "expected 10 representative samples, found $($samples.Count)"
}
$seenSamples = @{}
$coveredOperations = @{}
$coveredRisks = @{}
foreach ($sample in $samples) {
  if ($seenSamples.ContainsKey([string]$sample.id)) { throw "duplicate sample: $($sample.id)" }
  $seenSamples[[string]$sample.id] = $true
  $source = [string]$sample.source
  Assert-RelativeFile $source "sample $($sample.id)"
  if (-not (Test-Path -LiteralPath (Join-Path $root $source))) {
    throw "sample source does not exist: $source"
  }
  if (-not $moduleIds.ContainsKey([string]$sample.module_id)) {
    throw "sample module is absent from index: $($sample.module_id)"
  }
  foreach ($operation in @($sample.operations)) {
    if ([string]$operation -notin $operations) { throw "sample has unknown operation: $operation" }
    $coveredOperations[[string]$operation] = $true
  }
  foreach ($risk in @($sample.risks)) {
    if ([string]$risk -notmatch '^AI0[1-7]$') { throw "sample has invalid AI risk: $risk" }
    $coveredRisks[[string]$risk] = $true
  }
}
if (@($coveredOperations.Keys).Count -ne 5 -or @($coveredRisks.Keys).Count -ne 7) {
  throw 'sample matrix does not cover all operations and AI01-AI07'
}

$fixtureManifest = Read-Json (Join-Path $semanticRoot 'fixtures/manifest.json')
if ($fixtureManifest.schema -ne 'sico.semantic-query-fixtures.v0' -or $fixtureManifest.index -ne 'index.json') {
  throw 'unsupported semantic query fixture manifest'
}
$accepted = 0
$rejected = 0
$acceptedOperations = @{}
$maxResultBytes = 0
foreach ($fixture in @($fixtureManifest.fixtures)) {
  $requestPath = Join-Path $semanticRoot "fixtures/$($fixture.request)"
  $responsePath = Join-Path $semanticRoot "fixtures/$($fixture.response)"
  if (-not (Test-Path -LiteralPath $requestPath) -or -not (Test-Path -LiteralPath $responsePath)) {
    throw "fixture file is missing: $($fixture.name)"
  }
  $request = Read-Json $requestPath
  $response = Read-Json $responsePath
  $caught = $null
  try {
    Assert-Request $request $knownIds $operations $protocol "fixture $($fixture.name) request"
    Assert-Response $request $response $knownIds $symbolsById $root "fixture $($fixture.name) response"
  } catch {
    $caught = "$($_.Exception.Message) [$($_.ScriptStackTrace)]"
  }
  if ($fixture.expect -eq 'accept') {
    if ($null -ne $caught) { throw "fixture $($fixture.name) should pass: $caught" }
    if ($acceptedOperations.ContainsKey([string]$request.operation)) {
      throw "duplicate accepted operation fixture: $($request.operation)"
    }
    $acceptedOperations[[string]$request.operation] = $true
    $accepted++
    $maxResultBytes = [Math]::Max($maxResultBytes, [int]$response.budget.used_bytes)
  } elseif ($fixture.expect -eq 'reject') {
    if ($null -eq $caught) { throw "fixture $($fixture.name) should be rejected" }
    if ([string]::IsNullOrWhiteSpace([string]$fixture.error) -or -not $caught.Contains([string]$fixture.error)) {
      throw "fixture $($fixture.name) failed for the wrong reason: $caught"
    }
    $rejected++
  } else {
    throw "fixture $($fixture.name) has invalid expectation"
  }
}
if ($acceptedOperations.Count -ne 5) {
  throw 'accepted fixtures do not cover every operation exactly once'
}

Write-Output "SEMANTIC_QUERY_OK modules=$(@($index.modules).Count) symbols=$(@($index.symbols).Count) relations=$(@($index.relations).Count) samples=$($samples.Count) operations=$($operations.Count) fixtures=$(@($fixtureManifest.fixtures).Count) accepted=$accepted rejected=$rejected max_result_bytes=$maxResultBytes"
