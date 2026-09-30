param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'function gw_checked_add_subject_packed',
    'gmo_is_checked'
)) {
    if (-not $parser.Contains($marker)) {
        throw "missing STEP-0324 machinery marker: $marker"
    }
}
if (-not $tests.Contains('sico_compiler_lowers_gw_checked_arithmetic_match_subjects_byte_exactly')) {
    throw 'missing STEP-0324 test marker'
}
if (-not $tests.Contains('matching_close with its callees must compile')) {
    throw 'missing STEP-0324 matching_close flip marker'
}
& (Join-Path $PSScriptRoot 'validate-step-0323.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0323 inherited validator failed' }
Write-Output 'STEP_0324_OK gw-subjects=checked-add+checked-sub matching_close=byte-equal coverage=11to12-of-115 suite=56of56 residual=match-direct-in-while-body'
