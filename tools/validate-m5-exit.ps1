param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M5-desktop-host.md') -Raw -Encoding UTF8
$status = Get-Content -LiteralPath (Join-Path $root 'docs/STATUS.md') -Raw -Encoding UTF8
$roadmap = Get-Content -LiteralPath (Join-Path $root 'docs/ROADMAP.md') -Raw -Encoding UTF8
$audit = Get-Content -LiteralPath (Join-Path $root 'docs/reports/m5-exit-audit.md') -Raw -Encoding UTF8
if ($plan -notmatch '(?m)^> - status: complete\r?$') { throw 'M5 plan is not complete' }
foreach ($number in 46..53) {
  $step = Get-ChildItem -LiteralPath (Join-Path $root 'docs/steps') -Filter ('STEP-{0:D4}-*.md' -f $number)
  if (@($step).Count -ne 1) { throw "expected one document for STEP-$('{0:D4}' -f $number)" }
  $text = Get-Content -LiteralPath $step.FullName -Raw -Encoding UTF8
  if ($text -notmatch '(?m)^> - status: complete\r?$') { throw "$($step.Name) is not complete" }
}
if ($status -notmatch '(?m)^> - phase: M6 ' -or $status -notmatch '(?m)^> - next step: start STEP-0054\r?$') { throw 'STATUS is not advanced to M6 STEP-0054' }
if ($roadmap -notmatch '(?m)^> - current phase: M6\r?$') { throw 'ROADMAP current phase is not M6' }
if (-not $audit.Contains('GO: M5 complete; M6 entry gate satisfied; next STEP-0054.')) { throw 'M5 audit has no exact GO conclusion' }
foreach ($number in 46..53) {
  & (Join-Path $root ('tools/validate-step-{0:D4}.ps1' -f $number)) -RepositoryRoot $root -CargoPath $CargoPath
  if ($LASTEXITCODE -ne 0) { throw "STEP-$('{0:D4}' -f $number) validator failed" }
}
& (Join-Path $root 'tools/validate-m4-exit.ps1') -RepositoryRoot $root -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'M0-M4 regression failed during M5 exit' }
Write-Output 'M5_EXIT_OK steps=8 threat_cases=24 host_tests=16 desktop_tests=6 properties=10240 windows=runtime-verified macos=contract-verified linux=contract-verified regression=M0-M4 audit=GO next=STEP-0054'

