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
        'docs\steps\STEP-0204-m22-accepted-formatter-parity.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0204 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_formatter
if ($LASTEXITCODE -ne 0) { throw 'STEP-0204 accepted formatter differential failed' }

Write-Output 'STEP_0204_OK accepted=99 output=byte-exact idempotent=99 runner=real status=partial-S1'
