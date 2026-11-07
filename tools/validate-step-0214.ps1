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
        'docs\steps\STEP-0214-m22-corpus-bundle-baseline.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0214 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test bootstrap_bundle
if ($LASTEXITCODE -ne 0) { throw 'STEP-0214 bundle/manifest tests failed' }

& (Join-Path $root 'tools\update-m22-corpus.ps1') -RepositoryRoot $root -Verify
if ($LASTEXITCODE -ne 0) { throw 'STEP-0214 corpus oracle baseline drifted' }

# Descriptive summary of the frozen corpus census. The M22 corpus was re-frozen
# 215 -> 235 with STEP-0279..0283 (merge 7bcdc35), so the partition counts
# below are the measured 235-entry values (PowerShell ConvertFrom-Json over
# selfhost/corpus-v0.json, 2026-11-07): rust_format accepted 118 / refused 117;
# rust_check accepted 73 / refused 162. The gate's own assertions are the
# bootstrap_bundle test and the update-m22-corpus -Verify replay above.
Write-Output 'STEP_0214_OK corpus=235 format=118/117 check=73/162 bundle=strict evidence=internal-fixture'
