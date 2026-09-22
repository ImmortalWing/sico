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
    'M18-ai-application-pilots.md',
    'M19-production-engineering.md',
    'M20-platform-breadth-language-v1.md',
    'M21-developer-experience-and-stdlib.md',
    'M22-compiler-self-host.md',
    'M23-language-v1-batch-3.md',
    'M24-vision-closure-gui-application-pilot.md',
    'M25-release-v1-completion.md'
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

$bootstrapAdrPath = Join-Path $root 'docs\adr\ADR-0015-compiler-bootstrap-closure-v0.md'
if (-not (Test-Path -LiteralPath $bootstrapAdrPath)) { throw 'missing accepted M22 bootstrap ADR-0015' }
$bootstrapAdr = Get-Content -LiteralPath $bootstrapAdrPath -Raw -Encoding UTF8
foreach ($needle in 'status: accepted', 'A == B == C', 'Rust compiler remains', 'RFC-0015 canonical', 'cold and warm wall time') {
    if ($bootstrapAdr -notlike "*$needle*") { throw "ADR-0015 missing bootstrap contract: $needle" }
}
$m22Plan = Get-Content -LiteralPath (Join-Path $root 'docs\plans\M22-compiler-self-host.md') -Raw -Encoding UTF8
if ($m22Plan -notlike '*ADR-0015*' -or $m22Plan -notlike '*condition 5 closed*') {
    throw 'M22 plan must record bootstrap ADR-0015 entry condition closure'
}

$roadmap = Get-Content (Join-Path $root 'docs\ROADMAP.md') -Raw -Encoding UTF8
$headings = @(
    '## M14: Application-ready language baseline',
    '## M15: Web platform and UI controls',
    '## M16: Native Automation Host',
    '## M17: Vision and model package ecosystem',
    '## M18: Representative AI applications and external pilots',
    '## M19: Production engineering and release readiness',
    '## M20: Cross-platform runtime, language v1 and completion audit',
    '## M21: Developer experience, standard-library batch 2 and ecosystem activation',
    '## M22: Compiler self-host track',
    '## M23: Language v1 batch 3 — expression ergonomics',
    '## M24: Vision closure and GUI application pilot',
    '## M25: Release v1.0 — platform breadth, completion audit and product exit'
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
        '俄罗斯方块离线求解器',
        'M23 language v1 batch 3',
        'M24 vision closure + GUI application pilot',
        'M25 release v1.0 completion audit'
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

# The planning decision must not RESERVE future STEP numbers. A STEP number
# referenced by the roadmap is legitimate only when its execution record
# exists under docs/steps (implemented steps are no longer reservations).
$roadmapSteps = [regex]::Matches($roadmap, 'STEP-01(?:2[5-9]|[3-9][0-9])') |
    ForEach-Object { $_.Value } | Sort-Object -Unique
foreach ($step in $roadmapSteps) {
    $matches0 = Get-ChildItem (Join-Path $root 'docs\steps') -Filter ($step + '-*.md') -ErrorAction SilentlyContinue
    if (-not $matches0) {
        throw "roadmap references $step without an execution record under docs/steps (future STEP numbers must not be reserved by the planning decision)"
    }
}

Write-Output 'STEP_0124_OK milestones=M14-M25 steps=unreserved layers=language,web-ui,native-automation,vision-model,applications,production-engineering,platform-breadth-language-v1,compiler-self-host,language-v1-batch-3,vision-closure-gui-application,release-v1-completion'
