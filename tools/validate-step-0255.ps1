$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$plans = @{
    'M22' = Join-Path $root 'docs\plans\M22-compiler-self-host.md'
    'M23' = Join-Path $root 'docs\plans\M23-language-v1-batch-3.md'
    'M24' = Join-Path $root 'docs\plans\M24-vision-closure-block-game-pilot.md'
}
$text = @{}
foreach ($key in $plans.Keys) {
    $text[$key] = Get-Content -LiteralPath $plans[$key] -Raw -Encoding UTF8
    foreach ($section in 'Entry gate', 'Non-goals', 'Exit gates', 'no STEP numbers reserved') {
        if (-not $text[$key].Contains($section)) { throw "$key plan lost required contract text: $section" }
    }
}

# STEP-0255 refined sections and preserved M22 cross-references.
foreach ($marker in @(
    '### 3.1 Slice status',
    '### 3.2 Execution queue',
    'E-SH-IR-CALL-TARGET',
    'ADR-0015',
    'condition 5 closed',
    'STEP-0254'
)) {
    if (-not $text['M22'].Contains($marker)) { throw "M22 plan missing refinement marker: $marker" }
}
foreach ($marker in @(
    '## 8. Per-item work protocol',
    '## 9. Sequencing and dependency map',
    'smallest closed operator set',
    'D3 stands'
)) {
    if (-not $text['M23'].Contains($marker)) { throw "M23 plan missing refinement marker: $marker" }
}
foreach ($marker in @(
    '## 8. Gate-by-gate work breakdown',
    '### 8.4 Block-game pilot loop',
    'execute-one',
    '### 8.5 Error-injection corpus'
)) {
    if (-not $text['M24'].Contains($marker)) { throw "M24 plan missing refinement marker: $marker" }
}

if (-not (Test-Path -LiteralPath (Join-Path $root 'docs\steps\STEP-0255-m22-m24-plan-refinement.md'))) {
    throw 'missing STEP-0255 record document'
}
$roadmap = Get-Content -LiteralPath (Join-Path $root 'docs\ROADMAP.md') -Raw -Encoding UTF8
$status = Get-Content -LiteralPath (Join-Path $root 'docs\STATUS.md') -Raw -Encoding UTF8
$stepsIndex = Get-Content -LiteralPath (Join-Path $root 'docs\steps\README.md') -Raw -Encoding UTF8
foreach ($needle in @('STEP-0255')) {
    if (-not $roadmap.Contains($needle)) { throw "ROADMAP missing $needle pointer" }
    if (-not $status.Contains($needle)) { throw "STATUS missing $needle record" }
    if (-not $stepsIndex.Contains($needle)) { throw "steps index missing $needle row" }
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

Write-Output 'STEP_0255_OK m22=slice-status+execution-queue m23=per-item-protocol+sequencing m24=gate-breakdown+pilot-loop steps=unreserved'
