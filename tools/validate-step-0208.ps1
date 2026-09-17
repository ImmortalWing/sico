param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\compiler.sico',
        'selfhost\compiler_lexer.sico',
        'selfhost\compiler_parser.sico',
        'runner\sico-runner\tests\selfhost_compiler.rs',
        'docs\steps\STEP-0208-m22-modular-compiler-frontend.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0208 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_compiler
if ($LASTEXITCODE -ne 0) { throw 'STEP-0208 modular compiler frontend differential failed' }

Write-Output 'STEP_0208_OK compiler_modules=3 lexer=self-source-byte-exact parser=self-source-metadata-byte-exact ir_accept=6 mutation_refuse=1 core_wasm=byte-exact runner=real status=frontend-integrated'
