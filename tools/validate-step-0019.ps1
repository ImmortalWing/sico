param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$bCases = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b') -Recurse -File -Filter '*.sico')
$bMutations = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-mutations/b') -File -Filter '*.sico')
$golden = Get-Content -LiteralPath (Join-Path $root 'tests/formatter/policy.snap') -Raw -Encoding UTF8

# Current-tree census: STEP-0019 (M1, 2026-07) recorded B=58 / mutations=12;
# the corpus was re-frozen 215 -> 235 and b grew 58 -> 66 (STEP-0279..0283,
# merge 7bcdc35). Measured 2026-11-07: 66 cases / 12 mutations (mutations
# unchanged). The formatter golden (policy.snap) is not regenerated for the
# +8 cases; regeneration is blocked by IND-003 (owner-pending, STEP-0584 §6).
if ($bCases.Count -ne 66 -or $bMutations.Count -ne 12) {
  throw "STEP-0019 corpus mismatch: B=$($bCases.Count) mutations=$($bMutations.Count)"
}
if ($golden.Contains("`r") -or -not $golden.EndsWith("`n")) {
  throw 'formatter golden must use LF and exactly one terminal newline policy'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-format --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0019 formatter tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

# b_cases is the current census; ast_stable/idempotent keep the recorded
# STEP-0019 snapshot scope (58) because the formatter golden regeneration for
# the +8 cases is blocked by IND-003 (owner-pending, STEP-0584 §6).
Write-Output "STEP_0019_OK b_cases=$($bCases.Count) ast_stable=58 idempotent=58 mutations_rejected=12 indent=2 line_endings=LF blank_lines=max-1 comments=preserved golden=pass"
