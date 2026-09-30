param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$compiler = Get-Content -LiteralPath (Join-Path $root 'selfhost\compiler.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
$corpus = Get-Content -LiteralPath (Join-Path $root 'selfhost\codegen-corpus-v0.json') -Raw -Encoding UTF8
foreach ($marker in @(
    'function checked_sub_core_hex',
    'checked_sub_core_hex(source, words)'
)) {
    if (-not $compiler.Contains($marker)) {
        throw "missing STEP-0322 machinery marker: $marker"
    }
}
foreach ($marker in @(
    'checked-sub-i64',
    'checked-sub-u64',
    'checked_mul'
)) {
    if (-not $tests.Contains($marker) -and -not $corpus.Contains($marker)) {
        throw "missing STEP-0322 corpus/test marker: $marker"
    }
}
$parsed = $corpus | ConvertFrom-Json
if ($parsed.byte_equal_entries.Count -ne 16) { throw "STEP-0322 expects 16 byte-equal corpus entries, found $($parsed.byte_equal_entries.Count)" }
if ($parsed.typed_refusal_entries.Count -ne 7) { throw "STEP-0322 expects 7 typed-refusal corpus entries" }
& (Join-Path $PSScriptRoot 'validate-step-0321.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0321 inherited validator failed' }
Write-Output 'STEP_0322_OK checked-sub=2-byte-equal-families corpus=16-byte-equal+7-typed-refusal checked-mul=next-refusal-pin'
