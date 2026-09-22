$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

# STEP-0257 is superseded by STEP-0258 (owner directive deleted the
# block-game pilot target). This validator now pins the superseded state:
# the record document exists and says so, the UI-library prerequisite
# workstream (plan section 8.6.1) survives the re-designation, and the
# synchronized records still point at STEP-0257.
$planPath = Join-Path $root 'docs\plans\M24-vision-closure-gui-application-pilot.md'
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8
foreach ($section in 'Entry gate', 'Non-goals', 'Exit gates', 'no STEP numbers reserved') {
    if (-not $plan.Contains($section)) { throw 'M24 plan lost required contract text' }
}

# Whitespace-flattened view so wrapped prose can be asserted line-independently.
$flat = ($plan -replace '\s+', ' ')
foreach ($needle in @(
    '#### 8.6.1 Sico-language native UI library (prerequisite workstream)',
    'compiler-facing UI vocabulary authored in Sico source',
    'WinUI 3-like Fluent style',
    'RFC-0039',
    'strict companion model with no compiler-facing Sico binding',
    'no implementation claim is made until its own STEP lands with evidence'
)) {
    if ($flat -notlike "*$needle*") { throw "M24 plan lost the STEP-0257 UI-library workstream: $needle" }
}

$stepPath = Join-Path $root 'docs\steps\STEP-0257-m24-sico-native-ui-library-pilot-target.md'
if (-not (Test-Path -LiteralPath $stepPath)) { throw 'missing STEP-0257 record document' }
$stepDoc = Get-Content -LiteralPath $stepPath -Raw -Encoding UTF8
$stepFlat = ($stepDoc -replace '\s+', ' ')
foreach ($needle in @('superseded by STEP-0258')) {
    if ($stepFlat -notlike "*$needle*") { throw "STEP-0257 record missing superseded marker: $needle" }
}

$roadmap = Get-Content -LiteralPath (Join-Path $root 'docs\ROADMAP.md') -Raw -Encoding UTF8
$status = Get-Content -LiteralPath (Join-Path $root 'docs\STATUS.md') -Raw -Encoding UTF8
$stepsIndex = Get-Content -LiteralPath (Join-Path $root 'docs\steps\README.md') -Raw -Encoding UTF8
foreach ($pair in @(@('ROADMAP', $roadmap), @('STATUS', $status), @('steps index', $stepsIndex))) {
    if ($pair[1] -notlike '*STEP-0257*') { throw "$($pair[0]) missing STEP-0257 record" }
}

Write-Output 'STEP_0257_OK superseded-by=STEP-0258 ui-library-workstream=carried-over'
