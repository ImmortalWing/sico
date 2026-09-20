param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\parser.sico',
        'runner\sico-runner\tests\selfhost_parser.rs',
        'docs\steps\STEP-0235-m22-fixed-op-return-ir.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0235 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_parser
if ($LASTEXITCODE -ne 0) { throw 'STEP-0235 fixed-width operation return IR differential failed' }

Write-Output 'STEP_0235_OK positive=81 straight_lets=loop-driven bindings=operation+call+scalar-const+alias returns=name+scalar-const+fixed-literal+call+fixed-op verifier=independent equivalence=byte-exact runner=real status=partial-S4'
