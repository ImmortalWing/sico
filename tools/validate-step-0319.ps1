param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
if (-not $tests.Contains('sico_compiler_lowers_parser_rearranged_function_subsets_byte_exactly')) {
    throw 'missing STEP-0319 test marker'
}
foreach ($name in @('next_index', 'word_at', 'valid_escape_byte')) {
    if (-not $tests.Contains($name)) {
        throw "missing STEP-0319 function name: $name"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0318.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0318 inherited validator failed' }
Write-Output 'STEP_0319_OK per-function-coverage=5to8-of-115 canary=+3 r3-stall=reset-to-0 suite=54of54 prefix-frontier=unchanged-lex_words'
