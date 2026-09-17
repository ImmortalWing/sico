param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

$source = Get-Content -LiteralPath (Join-Path $root 'selfhost\compiler.sico') -Raw -Encoding UTF8
$test = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
foreach ($needle in '--emit-core-hex', '0061736d01000000') {
    if (-not $source.Contains($needle, [StringComparison]::Ordinal)) {
        throw "STEP-0199 compiler marker missing: $needle"
    }
}
foreach ($needle in 'sico_compiler_emits_deterministic_core_wasm_for_constant_slice', 'wasmparser::Validator') {
    if (-not $test.Contains($needle, [StringComparison]::Ordinal)) {
        throw "STEP-0199 test marker missing: $needle"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_compiler
if ($LASTEXITCODE -ne 0) { throw 'STEP-0199 deterministic Core Wasm differential failed' }

Write-Output 'STEP_0199_OK codegen=core-wasm equivalence=byte-exact validator=wasmparser shape=constant-return evidence=internal-fixture'
