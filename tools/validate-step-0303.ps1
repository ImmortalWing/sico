param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8

foreach ($marker in @(
    'let gmg_map_cell = local_binding_index(cell_names, gmg_map_name)',
    'read_local_instruction_json(base, gmg_map_ty,',
    'let gmg_key_pack = gw_rhs_packed(',
    'sico_compiler_lowers_map_get_from_local_byte_exactly',
    'sico_compiler_lowers_normalize_source_prefix_byte_exactly'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0303 marker: $marker"
    }
}

& (Join-Path $PSScriptRoot 'validate-step-0302.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0302 inherited validator failed' }
Write-Output 'STEP_0303_OK local-map-get=byte-exact historical-frontier=EXPRESSION current-frontier=MAIN-LET-RHS'
