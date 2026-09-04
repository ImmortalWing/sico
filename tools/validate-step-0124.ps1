param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)

$plans = @(
    'M14-application-ready-language.md',
    'M15-web-ui-platform.md',
    'M16-native-automation-host.md',
    'M17-vision-ml-ecosystem.md',
    'M18-ai-application-pilots.md'
)
foreach ($plan in $plans) {
    $path = Join-Path $root "docs\plans\$plan"
    if (-not (Test-Path -LiteralPath $path)) { throw "missing application roadmap plan: $plan" }
    $text = Get-Content -LiteralPath $path -Raw -Encoding UTF8
    foreach ($section in 'Entry gate', 'Non-goals', 'Exit gates') {
        if ($text -notlike "*$section*") { throw "$plan is missing section: $section" }
    }
    if ($text -notlike '*no STEP numbers reserved*') { throw "$plan must keep implementation STEP numbers unreserved" }
}

$roadmap = Get-Content (Join-Path $root 'docs\ROADMAP.md') -Raw -Encoding UTF8
$headings = @(
    '## M14: Application-ready language baseline',
    '## M15: Web platform and UI controls',
    '## M16: Native Automation Host',
    '## M17: Vision and model package ecosystem',
    '## M18: Representative AI applications and external pilots'
)
$previous = -1
foreach ($heading in $headings) {
    $index = $roadmap.IndexOf($heading, [StringComparison]::Ordinal)
    if ($index -lt 0 -or $index -le $previous) { throw "roadmap milestone missing or out of order: $heading" }
    $previous = $index
}
foreach ($needle in @(
        'M12 full GO + M13 closure',
        'M14 application-ready language',
        'M16 Native Automation',
        'M17 Vision/Model packages',
        '俄罗斯方块离线求解器'
    )) {
    if ($roadmap -notlike "*$needle*") { throw "roadmap invariant missing: $needle" }
}

$planIndex = Get-Content (Join-Path $root 'docs\plans\README.md') -Raw -Encoding UTF8
$docsIndex = Get-Content (Join-Path $root 'docs\README.md') -Raw -Encoding UTF8
foreach ($plan in $plans) {
    if ($planIndex -notlike "*$plan*" -or $docsIndex -notlike "*$plan*") {
        throw "plan is not linked from both indexes: $plan"
    }
}

$direction = Get-Content (Join-Path $root 'DIRECTION.md') -Raw -Encoding UTF8
$development = Get-Content (Join-Path $root 'DEVELOPMENT.md') -Raw -Encoding UTF8
$goal = Get-Content (Join-Path $root 'AGENT_GOAL.md') -Raw -Encoding UTF8
if ($direction -notlike '*所有者于 2026-09-04 进一步确认应用层节奏*') { throw 'owner roadmap decision is missing from DIRECTION.md' }
foreach ($heading in 'M14：Application-ready Language Baseline', 'M16：Native Automation Host', 'M18：Representative AI Applications') {
    if ($development -notlike "*$heading*") { throw "DEVELOPMENT milestone missing: $heading" }
}
foreach ($heading in 'M14：应用就绪语言基线', 'M16：Native Automation Host', 'M18：代表性 AI 应用与外部试点') {
    if ($goal -notlike "*$heading*") { throw "AGENT_GOAL milestone missing: $heading" }
}

$caseReadme = Get-Content (Join-Path $root '案例项目\俄罗斯方块消除\README.md') -Raw -Encoding UTF8
if ($caseReadme -notlike '*M14 纯 Sico 离线求解器、M16 capture/input、M17 vision、M18 完整应用试点*') {
    throw 'block-game milestone acceptance chain is missing'
}

if ($roadmap -match 'STEP-01(?:2[5-9]|[3-9][0-9])') {
    throw 'future M14-M18 implementation STEP numbers must not be reserved by this planning decision'
}

Write-Output 'STEP_0124_OK milestones=M14-M18 steps=unreserved layers=language,web-ui,native-automation,vision-model,applications'
