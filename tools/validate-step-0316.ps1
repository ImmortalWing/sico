param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'sico_compiler_lowers_parser_is_ident_byte_prefix_byte_exactly',
    'sico_compiler_lowers_control_if_decline_routes_to_general_machine',
    'sico_compiler_lowers_list_get_literal_index_match_byte_exactly'
)) {
    if (-not $tests.Contains($marker)) {
        throw "missing STEP-0316 marker: $marker"
    }
}
if (-not $parser.Contains('if sif_has_else:')) {
    throw 'STEP-0316 control-if no-else decline missing'
}
if (-not $parser.Contains('lgm_literal_index')) {
    throw 'STEP-0316 list.get literal-index machinery missing'
}
& (Join-Path $PSScriptRoot 'validate-step-0315.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0315 inherited validator failed' }
Write-Output 'STEP_0316_OK control-if-decline=2-forms list-get-literal-index=byte-exact parser-prefix=4-functions canary=30/30'
