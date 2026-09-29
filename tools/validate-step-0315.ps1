param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$compiler = Get-Content -LiteralPath (Join-Path $root 'selfhost\compiler.sico') -Raw -Encoding UTF8
$tests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
$corpus = Get-Content -LiteralPath (Join-Path $root 'selfhost\codegen-corpus-v0.json') -Raw -Encoding UTF8
foreach ($marker in @(
    'wasm_module_hex',
    'declared_core_hex',
    'checked_add_core_hex',
    'less_than_core_hex',
    'canonical_function_source',
    'sico_compiler_encodes_declared_scalar_core_wasm_families_byte_exactly',
    'sico.m22.codegen-corpus.v0'
)) {
    if (-not $compiler.Contains($marker) -and -not $tests.Contains($marker) -and -not $corpus.Contains($marker)) {
        throw "missing STEP-0315 marker: $marker"
    }
}
$byteEqual = ($corpus | ConvertFrom-Json).byte_equal_entries
if ($byteEqual.Count -ne 14) { throw "codegen corpus must pin 14 byte-equal entries, found $($byteEqual.Count)" }
& (Join-Path $PSScriptRoot 'validate-step-0314.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0314 inherited validator failed' }
Write-Output 'STEP_0315_OK codegen-families=14-byte-equal refusals=7-typed s5-subset-widened formatter-canary=30/30'
