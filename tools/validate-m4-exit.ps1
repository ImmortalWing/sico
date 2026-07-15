param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M4-sapp-runtime.md') -Raw -Encoding UTF8
$status = Get-Content -LiteralPath (Join-Path $root 'docs/STATUS.md') -Raw -Encoding UTF8
$roadmap = Get-Content -LiteralPath (Join-Path $root 'docs/ROADMAP.md') -Raw -Encoding UTF8
$audit = Get-Content -LiteralPath (Join-Path $root 'docs/reports/m4-exit-audit.md') -Raw -Encoding UTF8
if ($plan -notmatch '(?m)^> - status: complete\r?$') { throw 'M4 plan is not complete' }
foreach ($number in 38..45) {
  $step = Get-ChildItem -LiteralPath (Join-Path $root 'docs/steps') -Filter ('STEP-{0:D4}-*.md' -f $number)
  if (@($step).Count -ne 1) { throw "expected one document for STEP-$('{0:D4}' -f $number)" }
  $text = Get-Content -LiteralPath $step.FullName -Raw -Encoding UTF8
  if ($text -notmatch '(?m)^> - status: complete\r?$') { throw "$($step.Name) is not complete" }
}
if ($status -notmatch '(?m)^> - phase: M5 ' -or $status -notmatch '(?m)^> - next step: start STEP-0046\r?$') { throw 'STATUS is not advanced to M5 STEP-0046' }
if ($roadmap -notmatch '(?m)^> - current phase: M5\r?$') { throw 'ROADMAP current phase is not M5' }
if (-not $audit.Contains('GO: M4 complete; M5 entry gate satisfied; next STEP-0046.')) { throw 'M4 audit has no exact GO conclusion' }

foreach ($number in 38..45) {
  & (Join-Path $root ('tools/validate-step-{0:D4}.ps1' -f $number)) -RepositoryRoot $root -CargoPath $CargoPath
  if ($LASTEXITCODE -ne 0) { throw "STEP-$('{0:D4}' -f $number) validator failed" }
}
& (Join-Path $root 'tools/validate-m3-exit.ps1') -RepositoryRoot $root -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'M0-M3 regression failed during M4 exit' }

Write-Output 'M4_EXIT_OK steps=8 threat_cases=18 package_properties=3328 trust=ed25519-dev capability=closed storage=isolated runtime=limited cli=build-run-inspect cache=verified regression=M0-M3 audit=GO next=STEP-0046'
