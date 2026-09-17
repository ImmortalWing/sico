param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\corpus-v0.json',
        'runner\sico-runner\src\bootstrap.rs',
        'runner\sico-runner\tests\bootstrap_bundle.rs',
        'tools\update-m22-corpus.ps1',
        'docs\steps\STEP-0201-m22-corpus-bundle-baseline.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0201 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test bootstrap_bundle
if ($LASTEXITCODE -ne 0) { throw 'STEP-0201 bundle/manifest tests failed' }

& (Join-Path $root 'tools\update-m22-corpus.ps1') -RepositoryRoot $root -Verify
if ($LASTEXITCODE -ne 0) { throw 'STEP-0201 corpus oracle baseline drifted' }

Write-Output 'STEP_0201_OK corpus=215 format=99/116 check=65/150 bundle=strict evidence=internal-fixture'
