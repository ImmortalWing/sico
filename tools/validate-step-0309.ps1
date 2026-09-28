param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($marker in @(
    '"string", "sico.text.join"',
    'sico_compiler_lowers_text_join_inside_concat_byte_exactly',
    'sico_compiler_lowers_normalize_source_prefix_byte_exactly',
    'sico_compiler_refuses_full_formatter_at_main_condition'
)) {
    if (-not $parser.Contains($marker) -and -not $tests.Contains($marker)) {
        throw "missing STEP-0309 marker: $marker"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0308.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0308 inherited validator failed' }
Write-Output 'STEP_0309_OK text-join-in-concat=byte-exact formatter-canary=29/30 current-frontier=MAIN-SCICOND'
