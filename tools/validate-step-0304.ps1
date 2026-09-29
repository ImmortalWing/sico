param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'let gcp_l_callee = function_declaration_index(words, gcp_l_atom)',
    'set gcp_r_idx = next_index(gcp_l_comma, line_end)',
    'match U64.checked_add(gnext, gw_decimal_value(word_at(gr_parts, U64.literal(0)))):',
    'sico_compiler_lowers_user_call_as_compare_left_operand_byte_exactly',
    'sico_compiler_lowers_normalize_source_prefix_byte_exactly'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0304 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0303.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0303 inherited validator failed' }
Write-Output 'STEP_0304_OK compare-left-user-call=byte-exact return-parameter-ssa=stable current-frontier=MAIN-OK-RETURN'
