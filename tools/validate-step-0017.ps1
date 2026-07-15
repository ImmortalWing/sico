param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$bCases = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b') -Recurse -File -Filter '*.sico')
$snapshot = @(Get-Content -LiteralPath (Join-Path $root 'tests/parser/b-ast-shapes.txt') -Encoding UTF8 | Where-Object { $_.Trim() })
if ($bCases.Count -ne 54 -or $snapshot.Count -ne 54) {
  throw "B parser corpus/snapshot mismatch: cases=$($bCases.Count) snapshot=$($snapshot.Count)"
}
$accept = @($bCases | Where-Object { (Get-Content $_.FullName -Raw -Encoding UTF8).Contains('// expect: accept') }).Count
$semanticReject = @($bCases | Where-Object { (Get-Content $_.FullName -Raw -Encoding UTF8).Contains('// expect: reject(') }).Count
if ($accept -ne 25 -or $semanticReject -ne 29) {
  throw "unexpected B semantic partition: accept=$accept reject=$semanticReject"
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-syntax -p sico-parser --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0017 parser tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output "STEP_0017_OK b_cases=54 syntax_accept=54 designed_accept=$accept semantic_reject_not_executed=$semanticReject snapshots=$($snapshot.Count) lossless=pass"
