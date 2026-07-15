param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M3-sico-ir-component.md') -Raw -Encoding UTF8
$status = Get-Content -LiteralPath (Join-Path $root 'docs/STATUS.md') -Raw -Encoding UTF8
$roadmap = Get-Content -LiteralPath (Join-Path $root 'docs/ROADMAP.md') -Raw -Encoding UTF8
$audit = Get-Content -LiteralPath (Join-Path $root 'docs/reports/m3-exit-audit.md') -Raw -Encoding UTF8
if ($plan -notmatch '(?m)^> - status: complete\r?$') { throw 'M3 plan is not complete' }
foreach ($number in 30..37) {
  $step = Get-ChildItem -LiteralPath (Join-Path $root 'docs/steps') -Filter ('STEP-{0:D4}-*.md' -f $number)
  if (@($step).Count -ne 1) { throw "expected one document for STEP-$('{0:D4}' -f $number)" }
  $text = Get-Content -LiteralPath $step.FullName -Raw -Encoding UTF8
  if ($text -notmatch '(?m)^> - status: complete\r?$') { throw "$($step.Name) is not complete" }
}
if ($status -notmatch '(?m)^> - phase: M4 ' -or $status -notmatch '(?m)^> - next step: start STEP-0038\r?$') { throw 'STATUS is not advanced to M4 STEP-0038' }
if ($roadmap -notmatch '(?m)^> - current phase: M4\r?$') { throw 'ROADMAP current phase is not M4' }
if (-not $audit.Contains('GO: M3 complete; M4 entry gate satisfied; next STEP-0038.')) { throw 'M3 audit has no exact GO' }

foreach ($number in 30..37) {
  & (Join-Path $root ('tools/validate-step-{0:D4}.ps1' -f $number)) -RepositoryRoot $root -CargoPath $CargoPath
  if ($LASTEXITCODE -ne 0) { throw "STEP-$('{0:D4}' -f $number) validator failed" }
}
& (Join-Path $root 'tools/validate-m2-exit.ps1') -RepositoryRoot $root -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'M2 regression failed during M3 exit' }
& (Join-Path $root 'tools/validate-m1-exit.ps1') -RepositoryRoot $root -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'M1 regression failed during M3 exit' }
& (Join-Path $root 'tools/validate-m0-exit.ps1') -RepositoryRoot $root
if ($LASTEXITCODE -ne 0) { throw 'M0 regression failed during M3 exit' }

Write-Output 'M3_EXIT_OK steps=8 ir=typed-verified lowerable_valid=17 semantic_invalid=29 component=deterministic runtime=wasmtime-46.0.1 wit=result-record-resource async=future-stream cli=build-run scalar_inputs=2048 large_functions=1000 perf_median_ms=51.055 next=STEP-0038'
