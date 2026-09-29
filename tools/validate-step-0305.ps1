param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'let gtc_callee = function_declaration_index(words, gtc_atom)',
    'let gtc_call_pack = while_call_rhs_packed(',
    'sico_compiler_lowers_user_calls_inside_text_concat_byte_exactly',
    'sico_compiler_lowers_normalize_source_prefix_byte_exactly'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0305 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0304.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0304 inherited validator failed' }
Write-Output 'STEP_0305_OK text-concat-user-call-args=byte-exact historical-frontier=NESTED-TRIM-ARGUMENT current-frontier=MAIN-OK-RETURN'
