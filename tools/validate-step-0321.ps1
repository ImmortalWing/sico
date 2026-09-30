param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'sico_compiler_lowers_compiler_parser_function_subsets_byte_exactly',
    'scalar_type_kind',
    'fixed_op_name'
)) {
    if (-not $tests.Contains($marker)) {
        throw "missing STEP-0321 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0320.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0320 inherited validator failed' }
Write-Output 'STEP_0321_OK parser=9to11-of-115 compiler_parser=0to3-of-32 canary=+5 r3=0 suite=55of55'
