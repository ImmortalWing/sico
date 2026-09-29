param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    '"sico.bytes.is_utf8"',
    'parameter_type_word',
    '"field\":\"stdin\"',
    'sico_compiler_lowers_main_condition_byte_exactly',
    'sico_compiler_refuses_full_formatter_at_main_let_rhs'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0311 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0309.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0309 inherited validator failed' }
Write-Output 'STEP_0311_OK main-condition=byte-exact scriptinput-parameter=named formatter-canary=29/30 current-frontier=MAIN-LET-RHS'
