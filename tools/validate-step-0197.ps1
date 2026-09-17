param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\compiler.sico',
        'runner\sico-runner\tests\selfhost_compiler.rs',
        'docs\steps\STEP-0197-m22-typed-ir-verifier-seam.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0197 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_compiler
if ($LASTEXITCODE -ne 0) { throw 'STEP-0197 selfhost compiler differential failed' }

Write-Output 'STEP_0197_OK ir=canonical-json verifier=rust accepted=6 refused=1 runner=real evidence=internal-fixture'
