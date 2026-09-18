param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\formatter.sico',
        'selfhost\corpus-v0.json',
        'runner\sico-runner\tests\selfhost_formatter.rs',
        'docs\steps\STEP-0218-m22-formatter-refusal-gate.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0218 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_formatter
if ($LASTEXITCODE -ne 0) { throw 'STEP-0218 full-corpus formatter differential failed' }

Write-Output 'STEP_0218_OK corpus=215 accepted=99 byte_exact=99 idempotent=99 typed_refusal=116 runner=real status=S1-complete'
