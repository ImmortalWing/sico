param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'script_result_return_json',
    'gew_construct',
    'sico_compiler_lowers_error_return_variant_construct_byte_exactly',
    'sico_compiler_refuses_full_formatter_at_main_ok_return'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0313 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0312.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0312 inherited validator failed' }
Write-Output 'STEP_0313_OK error-return=byte-exact result-signature=named-pair formatter-canary=29/30 current-frontier=MAIN-OK-RETURN'
