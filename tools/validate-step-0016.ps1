param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

foreach ($required in @(
  'crates/sico-source/src/lib.rs',
  'crates/sico-lexer/src/lib.rs',
  'tests/lexical/contract-v0.json',
  'docs/steps/STEP-0016-source-span-lossless-lexer.md'
)) {
  if (-not (Test-Path -LiteralPath (Join-Path $root $required))) {
    throw "missing STEP-0016 artifact: $required"
  }
}

$contract = Get-Content -LiteralPath (Join-Path $root 'tests/lexical/contract-v0.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$contractCases = $contract.positive.Count + $contract.negative.Count
if ($contractCases -ne 21) {
  throw "expected 21 lexical contract cases, found $contractCases"
}
$bCases = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b') -Recurse -File -Filter '*.sico')
if ($bCases.Count -ne 58) {
  throw "expected 58 B cases, found $($bCases.Count)"
}

$sourceText = Get-Content -LiteralPath (Join-Path $root 'crates/sico-source/src/lib.rs') -Raw -Encoding UTF8
$lexerText = Get-Content -LiteralPath (Join-Path $root 'crates/sico-lexer/src/lib.rs') -Raw -Encoding UTF8
foreach ($required in @('SourceFile', 'LineIndex', 'TextRange', 'MAX_SOURCE_BYTES', 'BareCarriageReturn')) {
  if (-not $sourceText.Contains($required)) {
    throw "source implementation missing $required"
  }
}
foreach ($required in @('TokenKind', 'IdentifierNotNfc', 'MAX_TOKENS', 'reconstruct', 'all_b_canonical_files_are_lossless')) {
  if (-not $lexerText.Contains($required)) {
    throw "lexer implementation missing $required"
  }
}

$sourceTests = [regex]::Matches($sourceText, '(?m)^    #\[test\]$').Count
$lexerTests = [regex]::Matches($lexerText, '(?m)^    #\[test\]$').Count
if ($sourceTests -lt 6 -or $lexerTests -lt 8) {
  throw "insufficient STEP-0016 tests: source=$sourceTests lexer=$lexerTests"
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-source -p sico-lexer --quiet
  if ($LASTEXITCODE -ne 0) {
    throw 'STEP-0016 Rust tests failed'
  }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output "STEP_0016_OK contract=$contractCases b_cases=$($bCases.Count) source_tests=$sourceTests lexer_tests=$lexerTests utf8=pass line_index=pass lossless=pass limits=pass"
