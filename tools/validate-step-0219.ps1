param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\declaration_parser.sico',
        'selfhost\corpus-v0.json',
        'runner\sico-runner\tests\selfhost_declaration_parser.rs',
        'docs\steps\STEP-0219-m22-declaration-metadata.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0219 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_declaration_parser
if ($LASTEXITCODE -ne 0) { throw 'STEP-0219 accepted declaration metadata differential failed' }

Write-Output 'STEP_0219_OK accepted=99 shape=byte-exact declaration_metadata=byte-exact runner=real status=partial-S3'
