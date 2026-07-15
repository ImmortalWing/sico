param(
  [string]$RepositoryRoot
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
  $RepositoryRoot = Split-Path -Parent $PSScriptRoot
}
$root = (Resolve-Path $RepositoryRoot).Path
$scoreScript = Join-Path $PSScriptRoot 'score-ai-eval.ps1'
$prepareScript = Join-Path $PSScriptRoot 'prepare-ai-eval.ps1'
. (Join-Path $PSScriptRoot 'ai-eval-common.ps1')
$temporaryFiles = @(
  (Join-Path $env:TEMP "sico-ai-eval-pass-$([guid]::NewGuid().ToString('N')).json"),
  (Join-Path $env:TEMP "sico-ai-eval-fail-$([guid]::NewGuid().ToString('N')).json"),
  (Join-Path $env:TEMP "sico-ai-eval-packets-a-$([guid]::NewGuid().ToString('N')).json"),
  (Join-Path $env:TEMP "sico-ai-eval-packets-b-$([guid]::NewGuid().ToString('N')).json"),
  (Join-Path $env:TEMP "sico-ai-eval-model-run-$([guid]::NewGuid().ToString('N')).json")
)

try {
  & powershell -NoProfile -ExecutionPolicy Bypass -File $scoreScript `
    -RunPath (Join-Path $root 'ai-eval/fixtures/pass-run.json') `
    -OutputPath $temporaryFiles[0] `
    -RepositoryRoot $root | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw 'pass fixture scorer invocation failed'
  }
  $pass = Get-Content -LiteralPath $temporaryFiles[0] -Encoding UTF8 -Raw | ConvertFrom-Json
  if (-not $pass.synthetic -or $pass.official_comparison -or $pass.summary.score -ne 1 -or $pass.summary.fully_correct_attempts -ne 3) {
    throw 'pass fixture did not produce the expected synthetic perfect score'
  }

  & powershell -NoProfile -ExecutionPolicy Bypass -File $scoreScript `
    -RunPath (Join-Path $root 'ai-eval/fixtures/fail-run.json') `
    -OutputPath $temporaryFiles[1] `
    -RepositoryRoot $root | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw 'fail fixture scorer invocation failed'
  }
  $fail = Get-Content -LiteralPath $temporaryFiles[1] -Encoding UTF8 -Raw | ConvertFrom-Json
  $failureClasses = @($fail.results.failures | ForEach-Object { $_ } | Sort-Object -Unique)
  $expectedFailures = @('canonical-repair-mismatch', 'canonical-source-mismatch', 'forbidden-code-fence', 'invalid-json')
  if ($fail.summary.score -ne 0 -or $fail.summary.fully_correct_attempts -ne 0 -or
      ($failureClasses -join "`n") -cne ($expectedFailures -join "`n")) {
    throw 'fail fixture did not produce the expected failure classes'
  }

  $savedErrorActionPreference = $ErrorActionPreference
  $ErrorActionPreference = 'Continue'
  & powershell -NoProfile -ExecutionPolicy Bypass -File $scoreScript `
    -RunPath (Join-Path $root 'ai-eval/fixtures/invalid-model-run.json') `
    -OutputPath $temporaryFiles[1] `
    -RepositoryRoot $root 2>&1 | Out-Null
  $invalidModelExitCode = $LASTEXITCODE
  $ErrorActionPreference = $savedErrorActionPreference
  if ($invalidModelExitCode -eq 0) {
    throw 'invalid model metadata was unexpectedly accepted'
  }

  & powershell -NoProfile -ExecutionPolicy Bypass -File $prepareScript `
    -OutputPath $temporaryFiles[2] -RepositoryRoot $root | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw 'first packet generation failed'
  }
  & powershell -NoProfile -ExecutionPolicy Bypass -File $prepareScript `
    -OutputPath $temporaryFiles[3] -RepositoryRoot $root | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw 'second packet generation failed'
  }
  $firstHash = (Get-FileHash -LiteralPath $temporaryFiles[2] -Algorithm SHA256).Hash
  $secondHash = (Get-FileHash -LiteralPath $temporaryFiles[3] -Algorithm SHA256).Hash
  if ($firstHash -cne $secondHash) {
    throw 'prompt packet generation is not deterministic'
  }
  $packets = Get-Content -LiteralPath $temporaryFiles[2] -Encoding UTF8 -Raw | ConvertFrom-Json
  if ($packets.task_count -ne 96) {
    throw "expected 96 prompt packets, found $($packets.task_count)"
  }
  $packetText = Read-NormalizedText $temporaryFiles[2]
  if ($packetText -match 'semantic-cases/' -or $packetText -match 'syntax-candidates/' -or
      $packetText -match '(?m)^\s*// (case|expect|semantics):') {
    throw 'prompt packets leaked oracle paths or test metadata'
  }

  $taskById = @{}
  foreach ($task in @(Get-AiEvalTasks $root)) {
    $taskById[$task.task_id] = $task
  }
  $modelResponses = foreach ($taskId in @($packets.task_ids)) {
    $task = $taskById[[string]$taskId]
    if ($task.category -eq 'understanding') {
      $rawOutput = $task.expected_answer | ConvertTo-Json -Depth 20 -Compress
    }
    else {
      $rawOutput = Normalize-SicoSource (Read-NormalizedText (Join-Path $root $task.expected_path))
    }
    [pscustomobject][ordered]@{
      attempt_id = "model-path-$taskId"
      task_id = $taskId
      repetition = 1
      raw_output = $rawOutput
      input_tokens = 0
      output_tokens = 0
      cost = 0
      latency_ms = 0
    }
  }
  $modelRun = [pscustomobject][ordered]@{
    schema_version = 1
    protocol_id = 'sico-ai-eval-v1'
    run = [pscustomobject][ordered]@{
      run_id = 'transient-model-path-self-test'
      kind = 'model'
      synthetic = $false
      scope = 'smoke'
      task_ids = @($packets.task_ids)
      repetitions = 1
      model = [pscustomobject][ordered]@{
        provider = 'offline-self-test'
        name = 'no-model-called'
        version = 'transient-v1'
      }
      date = '2026-07-14'
      parameters = [pscustomobject][ordered]@{
        temperature = 0
        top_p = 1
        seed_supported = $false
        seed = $null
      }
      token_accounting = 'self-test-zero'
      cost_currency = 'USD'
      cost_total = 0
      rate_limits = 'no requests performed'
      prompt_packet_sha256 = (Get-Sha256 $temporaryFiles[2])
    }
    responses = @($modelResponses)
  }
  Write-Utf8NoBom $temporaryFiles[4] (($modelRun | ConvertTo-Json -Depth 20) + "`n")
  & powershell -NoProfile -ExecutionPolicy Bypass -File $scoreScript `
    -RunPath $temporaryFiles[4] `
    -PromptPacketPath $temporaryFiles[2] `
    -OutputPath $temporaryFiles[0] `
    -RepositoryRoot $root | Out-Null
  if ($LASTEXITCODE -ne 0) {
    throw 'complete model metadata path scorer invocation failed'
  }
  $modelPathScore = Get-Content -LiteralPath $temporaryFiles[0] -Encoding UTF8 -Raw | ConvertFrom-Json
  if ($modelPathScore.synthetic -or $modelPathScore.official_comparison -or
      $modelPathScore.summary.attempts -ne 96 -or $modelPathScore.summary.score -ne 1) {
    throw 'complete model metadata path did not produce the expected smoke result'
  }

  Write-Output 'AI_EVAL_TEST_OK pass_score=1 fail_score=0 invalid_model=rejected model_path=accepted packets=96 deterministic=true'
}
finally {
  foreach ($path in $temporaryFiles) {
    if (Test-Path -LiteralPath $path) {
      Remove-Item -LiteralPath $path -Force
    }
  }
}
