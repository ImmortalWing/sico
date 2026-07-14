param(
  [Parameter(Mandatory = $true)]
  [string]$RunPath,
  [Parameter(Mandatory = $true)]
  [string]$OutputPath,
  [string]$PromptPacketPath,
  [string]$RepositoryRoot
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
  $RepositoryRoot = Split-Path -Parent $PSScriptRoot
}
. (Join-Path $PSScriptRoot 'ai-eval-common.ps1')

function Assert-String([object]$Value, [string]$Name) {
  if ($Value -isnot [string] -or [string]::IsNullOrWhiteSpace($Value)) {
    throw "$Name must be a non-empty string"
  }
}

function Assert-Boolean([object]$Value, [string]$Name) {
  if ($Value -isnot [bool]) {
    throw "$Name must be a JSON boolean"
  }
}

function Assert-Integer([object]$Value, [string]$Name, [int]$Minimum) {
  if ($Value -is [bool] -or $Value -isnot [ValueType] -or [double]$Value -ne [Math]::Truncate([double]$Value) -or [int64]$Value -lt $Minimum) {
    throw "$Name must be an integer greater than or equal to $Minimum"
  }
}

function Assert-NonNegativeNumber([object]$Value, [string]$Name) {
  if ($null -eq $Value -or $Value -is [bool] -or -not ($Value -is [ValueType]) -or [double]$Value -lt 0) {
    throw "$Name must be a non-negative number"
  }
}

$root = (Resolve-Path $RepositoryRoot).Path
$protocol = Read-Utf8Json (Join-Path $root 'ai-eval/protocol.json')
$allTasks = @(Get-AiEvalTasks $root)
$taskById = @{}
foreach ($task in $allTasks) {
  $taskById[$task.task_id] = $task
}

$document = Read-Utf8Json ([IO.Path]::GetFullPath($RunPath))
if ((Get-ObjectProperty $document 'schema_version' 'run document') -ne 1) {
  throw 'unsupported run schema_version'
}
if ((Get-ObjectProperty $document 'protocol_id' 'run document') -ne $protocol.protocol_id) {
  throw 'run protocol_id does not match evaluator'
}
$run = Get-ObjectProperty $document 'run' 'run document'
$runIdValue = Get-ObjectProperty $run 'run_id' 'run'
Assert-String $runIdValue 'run.run_id'
$runId = [string]$runIdValue
$kindValue = Get-ObjectProperty $run 'kind' 'run'
Assert-String $kindValue 'run.kind'
$kind = [string]$kindValue
$syntheticValue = Get-ObjectProperty $run 'synthetic' 'run'
Assert-Boolean $syntheticValue 'run.synthetic'
$synthetic = [bool]$syntheticValue
$scopeValue = Get-ObjectProperty $run 'scope' 'run'
Assert-String $scopeValue 'run.scope'
$scope = [string]$scopeValue
if ($kind -notin @('model', 'fixture')) {
  throw "unsupported run.kind: $kind"
}
if (($kind -eq 'model' -and $synthetic) -or ($kind -eq 'fixture' -and -not $synthetic)) {
  throw 'run.kind and run.synthetic are inconsistent'
}
if ($kind -eq 'fixture') {
  Assert-String (Get-ObjectProperty $run 'fixture_name' 'fixture run') 'run.fixture_name'
}
if ($scope -notin @('full', 'smoke')) {
  throw "unsupported run.scope: $scope"
}

$taskIdsValue = Get-ObjectProperty $run 'task_ids' 'run'
if ($taskIdsValue -is [string] -or $taskIdsValue -isnot [Collections.IEnumerable]) {
  throw 'run.task_ids must be a JSON array'
}
$taskIds = @($taskIdsValue)
if ($taskIds.Count -eq 0) {
  throw 'run.task_ids cannot be empty'
}
$seenTaskIds = @{}
foreach ($taskIdValue in $taskIds) {
  Assert-String $taskIdValue 'run.task_ids entry'
  $taskId = [string]$taskIdValue
  if (-not $taskById.ContainsKey($taskId)) {
    throw "unknown run task_id: $taskId"
  }
  if ($seenTaskIds.ContainsKey($taskId)) {
    throw "duplicate run task_id: $taskId"
  }
  $seenTaskIds[$taskId] = $true
}
if ($scope -eq 'full') {
  $actualTaskSet = @($taskIds | Sort-Object) -join "`n"
  $expectedTaskSet = @($allTasks.task_id | Sort-Object) -join "`n"
  if ($actualTaskSet -cne $expectedTaskSet) {
    throw 'full run must include every v0 task exactly once in task_ids'
  }
}

$repetitionsValue = Get-ObjectProperty $run 'repetitions' 'run'
Assert-Integer $repetitionsValue 'run.repetitions' 1
$repetitions = [int]$repetitionsValue
if ($kind -eq 'model') {
  $model = Get-ObjectProperty $run 'model' 'model run'
  foreach ($name in @('provider', 'name', 'version')) {
    Assert-String (Get-ObjectProperty $model $name 'run.model') "run.model.$name"
  }
  if ([string](Get-ObjectProperty $model 'version' 'run.model') -eq 'latest') {
    throw 'run.model.version cannot be latest'
  }
  $date = [string](Get-ObjectProperty $run 'date' 'model run')
  if ($date -notmatch '^\d{4}-\d{2}-\d{2}$') {
    throw 'run.date must be YYYY-MM-DD'
  }
  $parameters = Get-ObjectProperty $run 'parameters' 'model run'
  Assert-NonNegativeNumber (Get-ObjectProperty $parameters 'temperature' 'run.parameters') 'run.parameters.temperature'
  $topP = Get-ObjectProperty $parameters 'top_p' 'run.parameters'
  Assert-NonNegativeNumber $topP 'run.parameters.top_p'
  if ([double]$topP -gt 1) {
    throw 'run.parameters.top_p must be less than or equal to 1'
  }
  $seedSupportedValue = Get-ObjectProperty $parameters 'seed_supported' 'run.parameters'
  Assert-Boolean $seedSupportedValue 'run.parameters.seed_supported'
  $seedSupported = [bool]$seedSupportedValue
  $seedProperty = $parameters.PSObject.Properties['seed']
  if ($null -eq $seedProperty) {
    throw "run.parameters is missing property 'seed'"
  }
  if ($seedSupported -and $null -eq $seedProperty.Value) {
    throw 'run.parameters.seed is required when seed_supported is true'
  }
  if ($seedSupported) {
    Assert-Integer $seedProperty.Value 'run.parameters.seed' 0
  }
  Assert-String (Get-ObjectProperty $run 'token_accounting' 'model run') 'run.token_accounting'
  Assert-String (Get-ObjectProperty $run 'cost_currency' 'model run') 'run.cost_currency'
  Assert-NonNegativeNumber (Get-ObjectProperty $run 'cost_total' 'model run') 'run.cost_total'
  Assert-String (Get-ObjectProperty $run 'rate_limits' 'model run') 'run.rate_limits'
  $packetHash = [string](Get-ObjectProperty $run 'prompt_packet_sha256' 'model run')
  if ($packetHash -notmatch '^[0-9a-f]{64}$') {
    throw 'run.prompt_packet_sha256 must be lowercase SHA-256'
  }
  if ($scope -eq 'full' -and $repetitions -lt [int]$protocol.minimum_model_repetitions) {
    throw "full model run requires at least $($protocol.minimum_model_repetitions) repetitions"
  }
  Assert-String $PromptPacketPath 'PromptPacketPath for model run'
  $actualPacketHash = Get-Sha256 ([IO.Path]::GetFullPath($PromptPacketPath))
  if ($actualPacketHash -cne $packetHash) {
    throw 'prompt packet hash does not match run metadata'
  }
  $packetDocument = Read-Utf8Json ([IO.Path]::GetFullPath($PromptPacketPath))
  if ($packetDocument.schema_version -ne 1 -or $packetDocument.protocol_id -ne $protocol.protocol_id) {
    throw 'prompt packet protocol does not match evaluator'
  }
  $packetTaskSet = @($packetDocument.task_ids | Sort-Object) -join "`n"
  $runTaskSet = @($taskIds | Sort-Object) -join "`n"
  if ($packetTaskSet -cne $runTaskSet) {
    throw 'prompt packet task_ids do not match run.task_ids'
  }
}

$responsesValue = Get-ObjectProperty $document 'responses' 'run document'
if ($responsesValue -is [string] -or $responsesValue -isnot [Collections.IEnumerable]) {
  throw 'responses must be a JSON array'
}
$responses = @($responsesValue)
$expectedAttempts = $taskIds.Count * $repetitions
if ($responses.Count -ne $expectedAttempts) {
  throw "expected $expectedAttempts responses, found $($responses.Count)"
}
$attemptIds = @{}
$attemptKeys = @{}
foreach ($response in $responses) {
  $attemptIdValue = Get-ObjectProperty $response 'attempt_id' 'response'
  Assert-String $attemptIdValue 'response.attempt_id'
  $attemptId = [string]$attemptIdValue
  if ($attemptIds.ContainsKey($attemptId)) {
    throw "duplicate attempt_id: $attemptId"
  }
  $attemptIds[$attemptId] = $true
  $taskIdValue = Get-ObjectProperty $response 'task_id' "response $attemptId"
  Assert-String $taskIdValue "response $attemptId.task_id"
  $taskId = [string]$taskIdValue
  if (-not $seenTaskIds.ContainsKey($taskId)) {
    throw "response references task outside run scope: $taskId"
  }
  $repetitionValue = Get-ObjectProperty $response 'repetition' "response $attemptId"
  Assert-Integer $repetitionValue "response $attemptId.repetition" 1
  $repetition = [int]$repetitionValue
  if ($repetition -lt 1 -or $repetition -gt $repetitions) {
    throw "invalid repetition for ${attemptId}: $repetition"
  }
  $attemptKey = "$taskId|$repetition"
  if ($attemptKeys.ContainsKey($attemptKey)) {
    throw "duplicate task/repetition response: $attemptKey"
  }
  $attemptKeys[$attemptKey] = $true
  $rawOutputValue = Get-ObjectProperty $response 'raw_output' "response $attemptId"
  if ($rawOutputValue -isnot [string]) {
    throw "response $attemptId.raw_output must be a JSON string"
  }
  if ($kind -eq 'model') {
    foreach ($name in @('input_tokens', 'output_tokens', 'cost', 'latency_ms')) {
      Assert-NonNegativeNumber (Get-ObjectProperty $response $name "response $attemptId") "response $attemptId.$name"
    }
  }
}

$results = @()
$groupSummary = @{}
$pointsEarned = 0
$maxPoints = 0
$fullyCorrectAttempts = 0
$usage = [ordered]@{ input_tokens = 0; output_tokens = 0; cost = 0.0; latency_ms = 0.0 }

foreach ($response in @($responses | Sort-Object task_id, repetition)) {
  $task = $taskById[[string]$response.task_id]
  $rawOutput = [string]$response.raw_output
  $failures = @()
  $earned = 0
  $maximum = 1

  if ([string]::IsNullOrWhiteSpace($rawOutput)) {
    $failures += 'no-output'
  }
  elseif ($rawOutput -match '(?m)^\s*```') {
    $failures += 'forbidden-code-fence'
  }

  if ($task.category -eq 'generation' -or $task.category -eq 'repair') {
    $expected = Normalize-SicoSource (Read-NormalizedText (Join-Path $root $task.expected_path))
    $actual = Normalize-SubmittedSicoSource $rawOutput
    if ($actual -ceq $expected -and $failures.Count -eq 0) {
      $earned = 1
    }
    else {
      $failure = if ($task.category -eq 'generation') { 'canonical-source-mismatch' } else { 'canonical-repair-mismatch' }
      if ($failure -notin $failures) {
        $failures += $failure
      }
    }
  }
  else {
    $expectedProperties = @($task.expected_answer.PSObject.Properties)
    $maximum = $expectedProperties.Count
    if ($failures.Count -eq 0) {
      try {
        $actualAnswer = $rawOutput | ConvertFrom-Json
        if ($actualAnswer -isnot [pscustomobject]) {
          throw 'understanding output must be a JSON object'
        }
        foreach ($property in $expectedProperties) {
          $actualProperty = $actualAnswer.PSObject.Properties[$property.Name]
          if ($null -eq $actualProperty) {
            if ('understanding-missing-field' -notin $failures) {
              $failures += 'understanding-missing-field'
            }
          }
          elseif ((ConvertTo-CanonicalValue $actualProperty.Value) -ceq (ConvertTo-CanonicalValue $property.Value)) {
            $earned += 1
          }
          elseif ('understanding-field-mismatch' -notin $failures) {
            $failures += 'understanding-field-mismatch'
          }
        }
        $expectedNames = @($expectedProperties.Name)
        $extraNames = @($actualAnswer.PSObject.Properties.Name | Where-Object { $_ -notin $expectedNames })
        if ($extraNames.Count -gt 0) {
          $failures += 'understanding-extra-field'
        }
      }
      catch {
        $failures = @('invalid-json')
        $earned = 0
      }
    }
  }

  $fullyCorrect = $earned -eq $maximum -and $failures.Count -eq 0
  if ($fullyCorrect) {
    $fullyCorrectAttempts += 1
  }
  $pointsEarned += $earned
  $maxPoints += $maximum
  $groupKey = "$($task.syntax)|$($task.category)"
  if (-not $groupSummary.ContainsKey($groupKey)) {
    $groupSummary[$groupKey] = [ordered]@{
      syntax = $task.syntax
      category = $task.category
      attempts = 0
      fully_correct = 0
      points_earned = 0
      max_points = 0
    }
  }
  $group = $groupSummary[$groupKey]
  $group.attempts += 1
  $group.fully_correct += [int]$fullyCorrect
  $group.points_earned += $earned
  $group.max_points += $maximum

  foreach ($name in @('input_tokens', 'output_tokens', 'cost', 'latency_ms')) {
    $property = $response.PSObject.Properties[$name]
    if ($null -ne $property) {
      $usage[$name] += [double]$property.Value
    }
  }

  $results += [pscustomobject][ordered]@{
    attempt_id = [string]$response.attempt_id
    task_id = $task.task_id
    repetition = [int]$response.repetition
    syntax = $task.syntax
    category = $task.category
    points_earned = $earned
    max_points = $maximum
    fully_correct = $fullyCorrect
    failures = @($failures | Sort-Object -Unique)
  }
}

$groups = foreach ($group in @($groupSummary.Values | Sort-Object syntax, category)) {
  [pscustomobject][ordered]@{
    syntax = $group.syntax
    category = $group.category
    attempts = $group.attempts
    fully_correct = $group.fully_correct
    points_earned = $group.points_earned
    max_points = $group.max_points
    score = [Math]::Round($group.points_earned / $group.max_points, 6)
  }
}

if ($kind -eq 'model') {
  $declaredCost = [double](Get-ObjectProperty $run 'cost_total' 'model run')
  if ([Math]::Abs($declaredCost - [double]$usage.cost) -gt 0.000001) {
    throw "run.cost_total does not match summed response cost: declared=$declaredCost summed=$($usage.cost)"
  }
}

$scoreDocument = [pscustomobject][ordered]@{
  schema_version = 1
  protocol_id = [string]$protocol.protocol_id
  run_id = $runId
  run_sha256 = Get-Sha256 ([IO.Path]::GetFullPath($RunPath))
  prompt_packet_sha256 = if ($kind -eq 'model') { Get-Sha256 ([IO.Path]::GetFullPath($PromptPacketPath)) } else { $null }
  synthetic = $synthetic
  official_comparison = ($kind -eq 'model' -and $scope -eq 'full' -and $repetitions -ge [int]$protocol.minimum_model_repetitions)
  summary = [pscustomobject][ordered]@{
    tasks = $taskIds.Count
    attempts = $responses.Count
    fully_correct_attempts = $fullyCorrectAttempts
    points_earned = $pointsEarned
    max_points = $maxPoints
    score = [Math]::Round($pointsEarned / $maxPoints, 6)
  }
  usage_totals = [pscustomobject]$usage
  groups = @($groups)
  results = @($results)
  not_measured = @($protocol.not_measured_without_compiler)
}

$fullOutputPath = [IO.Path]::GetFullPath($OutputPath)
$parent = Split-Path -Parent $fullOutputPath
if (-not (Test-Path -LiteralPath $parent)) {
  New-Item -ItemType Directory -Path $parent -Force | Out-Null
}
Write-Utf8NoBom $fullOutputPath (($scoreDocument | ConvertTo-Json -Depth 20) + "`n")
Write-Output "AI_EVAL_SCORE_OK run=$runId synthetic=$($synthetic.ToString().ToLowerInvariant()) attempts=$($responses.Count) fully_correct=$fullyCorrectAttempts score=$($scoreDocument.summary.score) path=$fullOutputPath"
