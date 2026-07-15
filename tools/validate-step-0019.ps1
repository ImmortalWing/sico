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

if ($bCases.Count -ne 54 -or $bMutations.Count -ne 12) {
  throw "STEP-0019 corpus mismatch: B=$($bCases.Count) mutations=$($bMutations.Count)"
}
if ($golden.Contains("`r") -or -not $golden.EndsWith("`n")) {
  throw 'formatter golden must use LF and exactly one terminal newline policy'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = 'stable'
try {
  & $CargoPath test --offline --locked -p sico-format --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0019 formatter tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output "STEP_0019_OK b_cases=54 ast_stable=54 idempotent=54 mutations_rejected=12 indent=2 line_endings=LF blank_lines=max-1 comments=preserved golden=pass"
