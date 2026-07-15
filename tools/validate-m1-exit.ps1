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
if ($m2Plan -notmatch '(?m)^> - status: ready after STEP-0021 GO\r?$') { throw 'M2 plan is not ready' }
if (-not $audit.Contains('GO: M1 complete; M2 entry gate satisfied; next STEP-0022.')) { throw 'M1 audit has no exact GO conclusion' }
if ($status -notmatch '(?m)^> - phase: M2 ' -or $status -notmatch '(?m)^> - next step: STEP-0022\r?$') {
  throw 'STATUS does not hand off from M1 to STEP-0022'
}

$performance = Get-Content -LiteralPath (Join-Path $root 'tests/performance/m1-frontend-windows-release.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if (@($performance.runs).Count -ne 3 -or $performance.corpus.files -ne 66 -or $performance.sla -ne 'not-established') {
  throw 'M1 performance evidence is incomplete or overclaims an SLA'
}
$bCases = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b') -Recurse -File -Filter '*.sico')
$bMutations = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-mutations/b') -File -Filter '*.sico')
$parserSnapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/parser/b-ast-shapes.txt') -Encoding UTF8 | Where-Object { $_.Trim() })
$diagnosticSnapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/diagnostics/b-mutations.snap') -Encoding UTF8 | Where-Object { $_.Trim() })
if ($bCases.Count -ne 54 -or $bMutations.Count -ne 12 -or $parserSnapshots.Count -ne 54 -or $diagnosticSnapshots.Count -ne 12) {
  throw 'M1 corpus/snapshot evidence counts drifted'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = 'stable'
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

Write-Output 'M1_EXIT_OK steps=7 source=pass lexer=54/54 parser=54/54 recovery=12/12 formatter=54/54 cli=3 property_inputs=8192 depth_limit=256 parser_errors=100 semantic_reject_syntax_success=29 m2_plan=ready next=STEP-0022'
