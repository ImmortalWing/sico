param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

foreach ($number in 22..29) {
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

$m2Plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M2-static-semantics.md') -Raw -Encoding UTF8
$m3Plan = Get-Content -LiteralPath (Join-Path $root 'docs/plans/M3-sico-ir-component.md') -Raw -Encoding UTF8
$audit = Get-Content -LiteralPath (Join-Path $root 'docs/reports/m2-exit-audit.md') -Raw -Encoding UTF8
$status = Get-Content -LiteralPath (Join-Path $root 'docs/STATUS.md') -Raw -Encoding UTF8
$cli = Get-Content -LiteralPath (Join-Path $root 'crates/sico-cli/src/lib.rs') -Raw -Encoding UTF8
if ($m2Plan -notmatch '(?m)^> - status: complete\r?$') { throw 'M2 plan is not complete' }
if ($m3Plan -notmatch '(?m)^> - status: (ready after STEP-0029 GO|in progress.*|complete)\r?$') { throw 'M3 plan is missing after M2 GO' }
if (-not $audit.Contains('GO: M2 complete; M3 entry gate satisfied; next STEP-0030.')) { throw 'M2 audit has no exact GO conclusion' }
if ($status -notmatch '(?m)^> - phase: M[3-9] ' -or $status -notmatch '(?m)^> - next step: (?:start )?STEP-00(3[0-9]|[4-9][0-9])\r?$') {
  throw 'STATUS does not preserve the post-M2 handoff'
}
if ($cli.Contains('type checker: unavailable') -or $cli.Contains('"type_checker": "unavailable"')) {
  throw 'CLI still contains the M1 semantic boundary'
}

$manifest = Get-Content -LiteralPath (Join-Path $root 'semantic-cases/manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$map = Get-Content -LiteralPath (Join-Path $root 'diagnostics/semantic-case-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($manifest.case_count -ne 54 -or $manifest.valid_count -ne 25 -or $manifest.invalid_count -ne 29 -or @($map.cases).Count -ne 29) {
  throw 'M2 semantic oracle counts drifted'
}

$performance = Get-Content -LiteralPath (Join-Path $root 'tests/performance/m2-semantics-windows-release.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if (@($performance.runs).Count -ne 3 -or $performance.sla -ne 'not-established' -or $performance.median.elapsed_ms -ne 1242.186) {
  throw 'M2 performance evidence is incomplete or overclaims an SLA'
}

foreach ($number in 22..29) {
  & (Join-Path $root ('tools/validate-step-{0:D4}.ps1' -f $number)) -RepositoryRoot $root -CargoPath $CargoPath
  if ($LASTEXITCODE -ne 0) { throw "STEP-$('{0:D4}' -f $number) validator failed" }
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
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

Write-Output 'M2_EXIT_OK steps=8 hir=54 semantic_valid=25 semantic_invalid=29 exact_primary=29 cli=text-json index_modules=10 queries=5 property_inputs=2048 diagnostic_cap=100 perf_median_ms=1242.186 next=STEP-0030'
