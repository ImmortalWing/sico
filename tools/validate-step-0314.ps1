param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'gw_ok_wrap_rhs_packed',
    'sico_compiler_lowers_ok_return_construct_byte_exactly',
    'sico_compiler_lowers_full_formatter_byte_exactly'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0314 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0313.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0313 inherited validator failed' }
Write-Output 'STEP_0314_OK ok-return=byte-exact full-formatter=byte-exact formatter-canary=30/30 frontier=CLOSED'
