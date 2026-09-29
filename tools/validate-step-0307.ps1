param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'function gw_text_list_binary_packed(',
    'return gw_text_list_binary_packed(',
    'sico_compiler_lowers_text_list_append_rhs_byte_exactly',
    'sico_compiler_lowers_normalize_source_prefix_byte_exactly'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0307 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0306.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0306 inherited validator failed' }
Write-Output 'STEP_0307_OK text-list-append=byte-exact historical-frontier=MAP-PUT-LITERAL current-frontier=MAIN-ERROR-RETURN'
