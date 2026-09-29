param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8

if ($parser.Contains('return "SKIP:GENERAL-WHILE-IF-REGION"')) {
    throw 'else/join while guard remains in parser.sico'
}
foreach ($marker in @(
    'sico_compiler_lowers_while_in_else_and_join_regions_byte_exactly',
    'sico_compiler_lowers_normalize_source_prefix_byte_exactly'
)) {
    if (-not $tests.Contains($marker)) { throw "missing STEP-0302 differential: $marker" }
}

& (Join-Path $PSScriptRoot 'validate-step-0262.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0262 inherited validator failed' }
Write-Output 'STEP_0302_OK else+join-while=byte-exact historical-frontier=CALL-ARGUMENT current-frontier=MAIN-LET-RHS'
