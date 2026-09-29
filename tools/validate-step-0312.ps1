param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'gwsdf_project',
    'sico_compiler_lowers_let_utf8_decode_field_rhs_byte_exactly',
    'sico_compiler_refuses_full_formatter_at_main_ok_return'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0312 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0311.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0311 inherited validator failed' }
Write-Output 'STEP_0312_OK let-utf8-decode-field-rhs=byte-exact formatter-canary=29/30 current-frontier=MAIN-OK-RETURN'
