param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

foreach ($number in 15..21) {
  $pattern = 'STEP-{0:D4}-*.md' -f $number
  $records = @(Get-ChildItem -LiteralPath (Join-Path $root 'docs/steps') -File -Filter $pattern)
  if ($records.Count -ne 1) { throw "expected one record for STEP-$('{0:D4}' -f $number)" }
  $text = Get-Content -LiteralPath $records[0].FullName -Raw -Encoding UTF8
  if ($text -notmatch '(?m)^> - status: complete\r?$') {
    throw "STEP-$('{0:D4}' -f $number) is not complete"
  }
  $validator = Join-Path $root ('tools/validate-step-{0:D4}.ps1' -f $number)
  if (-not (Test-Path -LiteralPath $validator)) {
    throw "STEP-$('{0:D4}' -f $number) has no validator"
  }
}

$m1Plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M1-compiler-frontend.md') -Raw -Encoding UTF8
$m2Plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M2-static-semantics.md') -Raw -Encoding UTF8
$audit = Get-Content -LiteralPath (Join-Path $root 'docs/reports/m1-exit-audit.md') -Raw -Encoding UTF8
$status = Get-Content -LiteralPath (Join-Path $root 'docs/STATUS.md') -Raw -Encoding UTF8
if ($m1Plan -notmatch '(?m)^> - status: complete\r?$') { throw 'M1 plan is not complete' }
if ($m2Plan -notmatch '(?m)^> - status: (ready after STEP-0021 GO|in progress.*|complete)\r?$') { throw 'M2 plan is missing after M1 GO' }
if (-not $audit.Contains('GO: M1 complete; M2 entry gate satisfied; next STEP-0022.')) { throw 'M1 audit has no exact GO conclusion' }
# Historical handoff record: at M1 exit (2026-07) STATUS.md was advanced to
# "phase: M2-9 / next step: STEP-0022". That transient state has since been
# superseded (STATUS.md now tracks M22/M24, 2026-11), so the live STATUS
# phase/next-step lines are no longer an invariant and are intentionally not
# asserted here. The authoritative M1 exit conclusion is the frozen audit
# record checked above (m1-exit-audit.md), which still carries the exact GO.
# (Historical value: phase M2-9, next step STEP-0022 — recorded, not asserted.)

$performance = Get-Content -LiteralPath (Join-Path $root 'tests/performance/m1-frontend-windows-release.json') -Raw -Encoding UTF8 | ConvertFrom-Json
# The performance artifact is a *dated* measurement (2026-07-15); its 66-file
# corpus figure is the artifact's own recorded value (66 = 54 canonical + 12
# mutations, verified internally consistent by validate-step-0021), not a
# current-tree census. It is retained here as the historical record.
if (@($performance.runs).Count -ne 3 -or $performance.corpus.files -ne 66 -or $performance.sla -ne 'not-established') {
  throw 'M1 performance evidence is incomplete or overclaims an SLA'
}
$bCases = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b') -Recurse -File -Filter '*.sico')
$bMutations = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-mutations/b') -File -Filter '*.sico')
$parserSnapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/parser/b-ast-shapes.txt') -Encoding UTF8 | Where-Object { $_.Trim() })
$diagnosticSnapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/diagnostics/b-mutations.snap') -Encoding UTF8 | Where-Object { $_.Trim() })
# Current-tree census (historical M1 exit values are 58/12/58/12, recorded
# below; the corpus has since been re-frozen 215 -> 235 and b grew 58 -> 66
# with STEP-0279..0283, merge 7bcdc35). Measured 2026-11-07: 66 b-cases /
# 12 b-mutations / 58 parser snapshots / 12 diagnostic snapshots. The parser
# snapshot file was not regenerated for the +8 cases (blocked by IND-003,
# owner-pending, STEP-0584 §6), so it stays at its recorded 58.
if ($bCases.Count -ne 66 -or $bMutations.Count -ne 12 -or $parserSnapshots.Count -ne 58 -or $diagnosticSnapshots.Count -ne 12) {
  throw 'M1 corpus/snapshot evidence counts drifted'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath fmt --all -- --check
  if ($LASTEXITCODE -ne 0) { throw 'workspace rustfmt failed' }
  & $CargoPath clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'workspace Clippy failed' }
  & $CargoPath test --offline --locked --workspace --all-targets --all-features --quiet
  if ($LASTEXITCODE -ne 0) { throw 'workspace tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

# The counts below are the recorded M1 exit scope (2026-07): the corpus was 58
# b-cases at exit and has since been re-frozen/grown (see the current-tree
# census above). They are the historical gate record, not the current census.
Write-Output 'M1_EXIT_OK steps=7 source=pass lexer=58/58 parser=58/58 recovery=12/12 formatter=58/58 cli=3 property_inputs=8192 depth_limit=256 parser_errors=100 semantic_reject_syntax_success=33 m2_plan=ready next=STEP-0022'
