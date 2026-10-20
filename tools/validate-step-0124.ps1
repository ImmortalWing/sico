param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
    [switch]$SelfTest
)

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
    'M25-release-v1-completion.md',
    'M26-pdf-document-processing.md'
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
if ($m22Plan -notmatch '(?m)^\| 5 \| Bootstrap architecture ADR accepted[^\r\n]*\*\*satisfied\*\*[^\r\n]*ADR-0015') {
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
    '## M25: Release v1.0 — platform breadth, completion audit and product exit',
    '## M26: PDF document processing'
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
        'M24 vision/native GUI pilot',
        'M25 v1.0 release verdict',
        'M26 PDF package and native GUI'
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

$routeTexts = @{
    case = Get-Content (Join-Path $root '案例项目\俄罗斯方块消除\README.md') -Raw -Encoding UTF8
    m22 = $m22Plan
    route = Get-Content (Join-Path $root 'docs\plans\M22-M26-route-replan-v1.md') -Raw -Encoding UTF8
    m23 = Get-Content (Join-Path $root 'docs\plans\M23-language-v1-batch-3.md') -Raw -Encoding UTF8
    m24 = Get-Content (Join-Path $root 'docs\plans\M24-vision-closure-gui-application-pilot.md') -Raw -Encoding UTF8
    m25 = Get-Content (Join-Path $root 'docs\plans\M25-release-v1-completion.md') -Raw -Encoding UTF8
    adr17 = Get-Content (Join-Path $root 'docs\adr\ADR-0017-selfhost-data-model-architecture-v0.md') -Raw -Encoding UTF8
}
function Assert-CurrentRoute([hashtable]$texts) {
    if ($texts.case -notmatch 'M14 纯 Sico 离线求解器验收 oracle' -or
        $texts.case -notmatch 'M18/M24.*方块游戏.*删除' -or
        $texts.case -match 'M18 完整应用试点') {
        throw 'current case README must retain the M14 oracle and retire the M18 block-game pilot'
    }
    if ($texts.adr17 -notmatch '(?m)^> - status: accepted \(Option A') {
        throw 'ADR-0017 must record the accepted R0 Option A direction'
    }
    if ($texts.route -notmatch 'A stalled\s+implementation STEP counts' -or
        $texts.route -notmatch 'Starting with W2\s+S3/S4 implementation' -or
        $texts.m22 -notmatch 'A stalled implementation STEP \*\*does count\*\*') {
        throw 'M22 W2 R3 must count stalled implementation STEPs'
    }
    $m23Sequence = [regex]::Match($texts.m23, '(?s)## 9\. Sequencing and dependency map.*').Value
    if ($texts.m23 -notmatch 'Route A after M22 S7 GO or Route B after an M22 R3' -or
        $m23Sequence -notmatch 'Route A' -or $m23Sequence -notmatch 'Route B') {
        throw 'M23 entry and sequencing must retain both implementation routes'
    }
    if ($texts.m24 -notmatch 'gate 2\s+remains NO-GO/deferred' -or
        $texts.m24 -notmatch 'M25 reruns\s+M0–M24') {
        throw 'M24 must defer accelerator gate 2 without real comparison and allow parallel closure'
    }
    if ($texts.m25 -notmatch 'v1\.0 release\s+GO/NO-GO' -or
        $texts.m25 -notmatch 'project §13 completion GO/NO-GO' -or
        $texts.m25 -notmatch 'M23 batch 3 is GO' -or
        $texts.m25 -notmatch 'M24 native GUI converter is GO') {
        throw 'M25 must separate release from project completion and gate release on M23/M24'
    }
}
Assert-CurrentRoute $routeTexts

if ($SelfTest) {
    function Assert-MutationRejected([string]$name, [string]$key, [string]$before, [string]$after) {
        $mutated = @{}
        foreach ($k in $routeTexts.Keys) { $mutated[$k] = $routeTexts[$k] }
        $mutated[$key] = $mutated[$key].Replace($before, $after)
        if ($mutated[$key] -eq $routeTexts[$key]) { throw "self-test mutation did not apply: $name" }
        $rejected = $false
        try { Assert-CurrentRoute $mutated } catch { $rejected = $true }
        if (-not $rejected) { throw "route validator accepted forbidden mutation: $name" }
    }
    Assert-MutationRejected 'old-M18-pilot' 'case' 'M18/M24 的方块游戏应用试点已由 STEP-0258 删除' 'M18 完整应用试点'
    Assert-MutationRejected 'M23-single-route' 'm23' 'Route B after an M22 R3 stop-loss declaration' 'implementation waits for M22 S7 only'
    Assert-MutationRejected 'M22-stalls-excluded' 'route' 'implementation STEP counts' 'implementation STEP does not count'
    Assert-MutationRejected 'M24-reference-only-GO' 'm24' 'gate 2 remains NO-GO/deferred' 'gate 2 remains GO'
    Assert-MutationRejected 'M25-conflated-verdict' 'm25' 'project §13 completion GO/NO-GO' 'complete-with-deferrals'
    Write-Output 'STEP_0124_NEGATIVE_OK cases=old-M18-pilot,M23-single-route,M22-stalls-excluded,M24-reference-only-GO,M25-conflated-verdict'
}

# The planning decision must not RESERVE future STEP numbers. A STEP number
# referenced by the roadmap is legitimate only when its execution record
# exists under docs/steps (implemented steps are no longer reservations).
$roadmapSteps = [regex]::Matches($roadmap, 'STEP-0(?:1(?:2[5-9]|[3-9][0-9])|2[0-9]{2})') |
    ForEach-Object { $_.Value } | Sort-Object -Unique
foreach ($step in $roadmapSteps) {
    $matches0 = Get-ChildItem (Join-Path $root 'docs\steps') -Filter ($step + '-*.md') -ErrorAction SilentlyContinue
    if (-not $matches0) {
        throw "roadmap references $step without an execution record under docs/steps (future STEP numbers must not be reserved by the planning decision)"
    }
}

Write-Output 'STEP_0124_OK milestones=M14-M26 steps=unreserved layers=language,web-ui,native-automation,vision-model,applications,production-engineering,platform-breadth-language-v1,compiler-self-host,language-v1-batch-3,vision-closure-gui-application,release-v1-completion,pdf-document-processing'
