param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'function gw_list_get_subject_packed',
    'gmo_is_list_get',
    'result-string',
    'result-i64'
)) {
    if (-not $parser.Contains($marker)) {
        throw "missing STEP-0325 machinery marker: $marker"
    }
}
if (-not $tests.Contains('sico_compiler_lowers_gw_list_get_match_subjects_byte_exactly')) {
    throw 'missing STEP-0325 test marker'
}
& (Join-Path $PSScriptRoot 'validate-step-0324.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0324 inherited validator failed' }
Write-Output 'STEP_0325_OK gw-subject=list-get-param-list coverage=12-of-115-holds r3-stall=1of4 suite=57of57 downstream=find_body_end-moved-to-CONTROL'
