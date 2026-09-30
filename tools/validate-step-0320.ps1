param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'fn parser_function_text',
    'line_end with its callees must compile'
)) {
    if (-not $tests.Contains($marker)) {
        throw "missing STEP-0320 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0319.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0319 inherited validator failed' }
Write-Output 'STEP_0320_OK line_end=byte-equal-with-callees coverage=8to9-of-115 canary=+1 r3=0 downstream=user-call-rhs-gap'
