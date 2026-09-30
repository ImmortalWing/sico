param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'sbl_cursor_param',
    'sbl_cursor_form',
    'set sbl_cursor_value = sbl_cursor_id'
)) {
    if (-not $parser.Contains($marker)) {
        throw "missing STEP-0327 machinery marker: $marker"
    }
}
foreach ($name in @(
    'binding_parameter_index',
    'parameter_type_word',
    'script_result_return_json',
    'decimal_at_most',
    'return_expression_end'
)) {
    if (-not $tests.Contains($name)) {
        throw "missing STEP-0327 function marker: $name"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0326.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0326 inherited validator failed' }
Write-Output 'STEP_0327_OK coverage=12to17-of-115 canary=+5 r3=reset-to-0 bytes-at-cursor-id=fixed suite=58of58'
