param(
  [Parameter(Mandatory = $true)]
  [string]$OutputPath,
  [ValidateSet('generation', 'understanding', 'repair')]
  [string[]]$Category,
  [ValidateSet('A0', 'B', 'C')]
  [string[]]$Syntax,
  [string]$RepositoryRoot
)

$ErrorActionPreference = 'Stop'
$hasCategoryFilter = $PSBoundParameters.ContainsKey('Category')
$hasSyntaxFilter = $PSBoundParameters.ContainsKey('Syntax')
if ([string]::IsNullOrWhiteSpace($RepositoryRoot)) {
  $RepositoryRoot = Split-Path -Parent $PSScriptRoot
}
. (Join-Path $PSScriptRoot 'ai-eval-common.ps1')

$root = (Resolve-Path $RepositoryRoot).Path
$protocol = Read-Utf8Json (Join-Path $root 'ai-eval/protocol.json')
$tasks = @(Get-AiEvalTasks $root)
if ($hasCategoryFilter) {
  $tasks = @($tasks | Where-Object { $_.category -in $Category })
}
if ($hasSyntaxFilter) {
  $tasks = @($tasks | Where-Object { $_.syntax -in $Syntax })
}
if ($tasks.Count -eq 0) {
  throw 'filters selected no tasks'
}

$packets = foreach ($task in $tasks) {
  [pscustomobject][ordered]@{
    task_id = $task.task_id
    pair_id = $task.pair_id
    case_id = $task.case_id
    category = $task.category
    syntax = $task.syntax
    system_prompt = [string]$protocol.system_prompt
    user_prompt = New-AiEvalUserPrompt $root $task
  }
}

$packetSet = [pscustomobject][ordered]@{
  schema_version = 1
  protocol_id = [string]$protocol.protocol_id
  task_count = $packets.Count
  task_ids = @($packets.task_id)
  packets = @($packets)
}

$fullOutputPath = [IO.Path]::GetFullPath($OutputPath)
$parent = Split-Path -Parent $fullOutputPath
if (-not (Test-Path -LiteralPath $parent)) {
  New-Item -ItemType Directory -Path $parent -Force | Out-Null
}
$json = $packetSet | ConvertTo-Json -Depth 20
Write-Utf8NoBom $fullOutputPath ($json + "`n")
$hash = Get-Sha256 $fullOutputPath
Write-Output "AI_EVAL_PACKETS_OK tasks=$($packets.Count) sha256=$hash path=$fullOutputPath"
