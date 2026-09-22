$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

# STEP-0258 is documentation-only: assert the renamed M24 plan carries the
# GUI format-converter pilot and the deletion markers, that every
# synchronized record was re-pointed, then re-run the standing planning
# contract.
$planPath = Join-Path $root 'docs\plans\M24-vision-closure-gui-application-pilot.md'
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8
foreach ($section in 'Entry gate', 'Non-goals', 'Exit gates', 'no STEP numbers reserved') {
    if (-not $plan.Contains($section)) { throw 'M24 plan lost required contract text' }
}

$flat = ($plan -replace '\s+', ' ')
foreach ($needle in @(
    '# M24 Vision closure and GUI application pilot',
    '### 8.4 Format-converter pilot',
    '### 8.5 Format-converter evidence contract',
    'converts JPEG',
    'no codec I/O (honest gap)',
    'byte-stable for fixed input',
    'no STEP number is reserved',
    'Deleted-content note',
    'M16 real-loop evidence stands at its M16 GO level',
    '#### 8.6.1 Sico-language native UI library (prerequisite workstream)',
    'the M14 solver acceptance oracle'
)) {
    if ($flat -notlike "*$needle*") { throw "M24 plan missing STEP-0258 marker: $needle" }
}

$roadmap = Get-Content -LiteralPath (Join-Path $root 'docs\ROADMAP.md') -Raw -Encoding UTF8
foreach ($needle in @(
    '## M24: Vision closure and GUI application pilot',
    'GUI application pilot',
    'STEP-0258',
    'M24 vision closure + GUI application pilot'
)) {
    if ($roadmap -notlike "*$needle*") { throw "ROADMAP missing STEP-0258 marker: $needle" }
}

$m18 = Get-Content -LiteralPath (Join-Path $root 'docs\plans\M18-ai-application-pilots.md') -Raw -Encoding UTF8
$m18Flat = ($m18 -replace '\s+', ' ')
foreach ($needle in @(
    'GUI format converter authored in Sico',
    'Application pilot acceptance path (re-designated 2026-09-22)',
    'deleted by owner directive'
)) {
    if ($m18Flat -notlike "*$needle*") { throw "M18 plan missing STEP-0258 marker: $needle" }
}

$goal = Get-Content -LiteralPath (Join-Path $root 'AGENT_GOAL.md') -Raw -Encoding UTF8
# Non-ASCII needle built from code points: PS 5.1 reads this BOM-less
# script in the legacy codepage, so literal CJK would mojibake.
$goalNeedle = 'M24' + [string][char]0xFF1A + ([string][char]0x89C6) + ([string][char]0x89C9) + ([string][char]0x6536) + ([string][char]0x53E3) + ([string][char]0x4E0E) + ' GUI ' + ([string][char]0x5E94) + ([string][char]0x7528) + ([string][char]0x8BD5) + ([string][char]0x70B9)
if ($goal -notlike "*$goalNeedle*") { throw 'AGENT_GOAL M24 section not re-pointed' }
$direction = Get-Content -LiteralPath (Join-Path $root 'DIRECTION.md') -Raw -Encoding UTF8
if ($direction -notlike '*STEP-0258*') { throw 'DIRECTION missing STEP-0258 record' }
$m25 = Get-Content -LiteralPath (Join-Path $root 'docs\plans\M25-release-v1-completion.md') -Raw -Encoding UTF8
if ($m25 -notlike '*M24 application-pilot gate explicit*') { throw 'M25 entry gate not re-pointed' }

$status = Get-Content -LiteralPath (Join-Path $root 'docs\STATUS.md') -Raw -Encoding UTF8
$stepsIndex = Get-Content -LiteralPath (Join-Path $root 'docs\steps\README.md') -Raw -Encoding UTF8
foreach ($pair in @(@('STATUS', $status), @('steps index', $stepsIndex))) {
    if ($pair[1] -notlike '*STEP-0258*') { throw "$($pair[0]) missing STEP-0258 record" }
}
if ($stepsIndex -notlike '*superseded*') { throw 'steps index missing STEP-0257 superseded marker' }

# The case-project README path is non-ASCII; build it from Unicode code
# points so this validator source stays ASCII-only (Windows PowerShell 5.1
# reads BOM-less scripts in the legacy codepage).
$codePoints = @(0x6848, 0x4F8B, 0x9879, 0x76EE, 0x005C, 0x4FC4, 0x7F57, 0x65AF, 0x65B9, 0x5757, 0x6D88, 0x9664)
$caseRelative = (($codePoints | ForEach-Object { [string][char]$_ }) -join '') + '\README.md'
$caseReadmePath = Join-Path $root $caseRelative
if (-not (Test-Path -LiteralPath $caseReadmePath)) { throw "missing case README: $caseRelative" }
$caseReadme = Get-Content -LiteralPath $caseReadmePath -Raw -Encoding UTF8
if ($caseReadme -notlike '*GUI*') { throw 'case README missing the re-designated pilot section' }
if ($caseReadme -notlike '*STEP-0138*') { throw 'case README lost the M14 solver-oracle record' }

if (-not (Test-Path -LiteralPath (Join-Path $root 'docs\steps\STEP-0258-m24-pilot-redesignation-gui-format-converter.md'))) {
    throw 'missing STEP-0258 record document'
}

Push-Location $root
try {
    # A contract failure inside validate-step-0124.ps1 throws and stops
    # this validator with it.
    & (Join-Path $PSScriptRoot 'validate-step-0124.ps1') -RepositoryRoot $root

    git diff --check
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
}
finally {
    Pop-Location
}

Write-Output 'STEP_0258_OK m24=gui-format-converter-pilot block-game=deleted gates=unchanged steps=unreserved'
