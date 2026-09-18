param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\tokens.sico',
        'selfhost\corpus-v0.json',
        'runner\sico-runner\tests\selfhost_tokens.rs',
        'docs\steps\STEP-0215-m22-lossless-lexer-corpus.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0215 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_tokens
if ($LASTEXITCODE -ne 0) { throw 'STEP-0215 full-corpus lexer differential failed' }

Write-Output 'STEP_0215_OK corpus=215 lexer=lossless fields=kind,start,end,text equivalence=byte-exact runner=real'
