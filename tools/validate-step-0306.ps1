param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    'function gw_text_trim_user_call_packed(',
    'let gtc_trim_pack = gw_text_trim_user_call_packed(',
    'sico_compiler_lowers_trimmed_user_call_inside_text_concat_byte_exactly',
    'sico_compiler_lowers_normalize_source_prefix_byte_exactly'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0306 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0305.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0305 inherited validator failed' }
Write-Output 'STEP_0306_OK nested-trim-user-call=byte-exact historical-frontier=LIST-APPEND current-frontier=MAIN-LET-RHS'
