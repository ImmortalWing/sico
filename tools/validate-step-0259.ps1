$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot

# STEP-0259 is documentation-only: assert the M18 audit-addendum
# registration, the synchronized records, then re-run the standing
# planning contract.
$m18Path = Join-Path $root 'docs\plans\M18-ai-application-pilots.md'
$m18 = Get-Content -LiteralPath $m18Path -Raw -Encoding UTF8
$m18Flat = ($m18 -replace '\s+', ' ')
foreach ($needle in @(
    'Addendum registration (owner-confirmed 2026-09-22, STEP-0259)',
    'no M18 rework is required',
    'M18 exit-audit addendum',
    'flips portfolio item 4 from NO-GO to GO',
    'external pilot NO-GO is untouched by the addendum'
)) {
    if ($m18Flat -notlike "*$needle*") { throw "M18 plan missing STEP-0259 marker: $needle" }
}

$roadmap = Get-Content -LiteralPath (Join-Path $root 'docs\ROADMAP.md') -Raw -Encoding UTF8
if ($roadmap -notlike '*STEP-0259*') { throw 'ROADMAP missing STEP-0259 record' }
$status = Get-Content -LiteralPath (Join-Path $root 'docs\STATUS.md') -Raw -Encoding UTF8
if ($status -notlike '*STEP-0259*') { throw 'STATUS missing STEP-0259 record' }
$stepsIndex = Get-Content -LiteralPath (Join-Path $root 'docs\steps\README.md') -Raw -Encoding UTF8
if ($stepsIndex -notlike '*STEP-0259*') { throw 'steps index missing STEP-0259 row' }
if (-not (Test-Path -LiteralPath (Join-Path $root 'docs\steps\STEP-0259-m18-audit-addendum-registration.md'))) {
    throw 'missing STEP-0259 record document'
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

Write-Output 'STEP_0259_OK m18=no-rework addendum=registered-on-gate5 external-pilot=untouched'
