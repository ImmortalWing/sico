param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'if is_word(fixed_literal_argument(words, gmp_value_idx), "u64"):',
    'sico_compiler_lowers_map_put_fixed_literal_rhs_byte_exactly',
    'sico_compiler_lowers_normalize_source_before_final_join_byte_exactly',
    'sico_compiler_lowers_normalize_source_prefix_byte_exactly'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0308 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0307.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0307 inherited validator failed' }
Write-Output 'STEP_0308_OK map-put-u64-literal=byte-exact historical-frontier=TEXT-JOIN current-frontier=MAIN-ERROR-RETURN'
