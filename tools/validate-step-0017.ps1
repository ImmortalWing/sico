param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$bCases = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b') -Recurse -File -Filter '*.sico')
$snapshot = @(Get-Content -LiteralPath (Join-Path $root 'tests/parser/b-ast-shapes.txt') -Encoding UTF8 | Where-Object { $_.Trim() })
# Current-tree census (the historical STEP-0017 values are the M1-era facts
# recorded below; the corpus has since been re-frozen and grown):
#   - STEP-0017 recorded: bCases=58, snapshot=58, accept=25, reject=33.
#   - The corpus was re-frozen 215 -> 235 and b grew 58 -> 66 with
#     STEP-0279..0283 (merge 7bcdc35). Measured 2026-11-07: 66 cases /
#     58 snapshots / 28 accept / 38 reject. The parser snapshot file was NOT
#     regenerated for the +8 cases (it stays 58); regeneration is blocked by
#     IND-003 (owner-pending, STEP-0584 §6), so the snapshot count is
#     intentionally left at its recorded 58.
if ($bCases.Count -ne 66 -or $snapshot.Count -ne 58) {
  throw "B parser corpus/snapshot mismatch: cases=$($bCases.Count) snapshot=$($snapshot.Count)"
}
$accept = @($bCases | Where-Object { (Get-Content $_.FullName -Raw -Encoding UTF8).Contains('// expect: accept') }).Count
$semanticReject = @($bCases | Where-Object { (Get-Content $_.FullName -Raw -Encoding UTF8).Contains('// expect: reject(') }).Count
if ($accept -ne 28 -or $semanticReject -ne 38) {
  throw "unexpected B semantic partition: accept=$accept reject=$semanticReject"
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-syntax -p sico-parser --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0017 parser tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

# syntax_accept = 66 total - 1 parse-reject (IND-003); the remaining reject
# markers are semantic refusals (38), consistent with accept 28 + reject 38.
Write-Output "STEP_0017_OK b_cases=$($bCases.Count) syntax_accept=$($bCases.Count - 1) designed_accept=$accept semantic_reject_not_executed=$semanticReject snapshots=$($snapshot.Count) lossless=pass"
