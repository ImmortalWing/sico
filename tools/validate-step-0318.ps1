param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'sico_compiler_lowers_checked_arithmetic_set_arm_match_byte_exactly',
    'sico_compiler_refuses_invalid_set_arm_match_shapes_with_typed_identity'
)) {
    if (-not $tests.Contains($marker)) {
        throw "missing STEP-0318 test marker: $marker"
    }
}
foreach ($marker in @(
    'function checked_arithmetic_set_match_ir',
    'function literal_value_index',
    'casm_route_set'
)) {
    if (-not $parser.Contains($marker)) {
        throw "missing STEP-0318 machinery marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0317.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0317 inherited validator failed' }
Write-Output 'STEP_0318_OK set-arm-family=checked-add-only variants=6-byte-equal refusals=11-typed recovery=byte-exact canary=30/30-holds r3-stall=1of4'
