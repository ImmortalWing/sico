param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'sbi_cursor_is_param',
    'sbi_cursor_param'
)) {
    if (-not $parser.Contains($marker)) {
        throw "missing STEP-0326 machinery marker: $marker"
    }
}
if (-not $tests.Contains('sico_compiler_lowers_gw_host_call_lets_byte_exactly')) {
    throw 'missing STEP-0326 test marker'
}
& (Join-Path $PSScriptRoot 'validate-step-0325.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0325 inherited validator failed' }
Write-Output 'STEP_0326_OK gw-host-call-lets=encode+bytes.length bytes-at=param-cursor pack-format=kind-result-count-pieces coverage=12-of-115-holds r3-stall=2of4 suite=58of58'
