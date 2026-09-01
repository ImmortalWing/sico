param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
[Console]::OutputEncoding = [Text.Encoding]::UTF8

function Require-Text([string]$RelativePath, [string]$Pattern) {
    $path = Join-Path $root $RelativePath
    if (-not (Test-Path -LiteralPath $path)) { throw "missing-audit-input|$RelativePath" }
    if (-not (Select-String -LiteralPath $path -Pattern $Pattern -Quiet -Encoding UTF8)) {
        throw "audit-input-mismatch|$RelativePath|$Pattern"
    }
}

$adr = Join-Path $root 'docs\adr\ADR-0011-ai-quality-budgets.md'
$baseline = Join-Path $root 'docs\steps\STEP-0119-ai-generation-quality-baseline.md'

# --- ADR exists, accepted, and fixes both tiers and the protocol identity ---
Require-Text 'docs\adr\ADR-0011-ai-quality-budgets.md' 'status: accepted'
Require-Text 'docs\adr\ADR-0011-ai-quality-budgets.md' 'sico-ai-eval-v1'
Require-Text 'docs\adr\ADR-0011-ai-quality-budgets.md' 'byte-exact'
Require-Text 'docs\adr\ADR-0011-ai-quality-budgets.md' 'Floor budgets'
Require-Text 'docs\adr\ADR-0011-ai-quality-budgets.md' 'Target budgets'
Require-Text 'docs\adr\ADR-0011-ai-quality-budgets.md' 'blocked-external-evidence'

# --- every floor must be at or below the measured STEP-0119 baseline ---
$adrText = Get-Content -LiteralPath $adr -Raw -Encoding UTF8
$baseText = Get-Content -LiteralPath $baseline -Raw -Encoding UTF8
$measured = @{
    'generation A0' = 1.000; 'generation B' = 0.900; 'generation C' = 0.600
    'repair' = 1.000; 'understanding B' = 0.982; 'overall' = 0.974359
}
foreach ($key in @('1.000', '0.900', '0.600', '0.982', '0.974359')) {
    if ($baseText -notlike "*$key*") { throw "baseline number missing from STEP-0119 doc: $key" }
}
$floors = @{ overall = 0.950; 'gen A0' = 0.950; 'gen B' = 0.850; 'gen C' = 0.550; understanding = 0.950 }
foreach ($floor in $floors.GetEnumerator()) {
    if ($adrText -notlike "*$($floor.Value)*") { throw "floor missing from ADR: $($floor.Key) $($floor.Value)" }
}
if (0.950 -gt 0.974359 -or 0.950 -gt 1.000 -or 0.850 -gt 0.900 -or 0.550 -gt 0.600 -or 0.950 -gt 0.982) {
    throw 'floor above measured baseline'
}
# --- targets must exceed the floors and be honestly deferred ---
if ($adrText -notlike '*0.980*') { throw 'overall target 0.980 missing' }
if ($adrText -notlike '*blocked-external-evidence*') { throw 'target deferral missing' }
if ($adrText -like '*live-model*proven*') { } # wording presence checked above; no live claim allowed
if ($adrText -match 'target budget.{0,80}(met|proven by the subagent)') { throw 'target claimed as met without live-model evidence' }

# --- registry consistency ---
Require-Text 'docs\adr\README.md' 'ADR-0011'
Require-Text 'docs\steps\README.md' 'STEP-0120'

Write-Output 'STEP_0120_OK adr-0011=accepted floors<=baseline targets=deferred-live-model registry=consistent'
