param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\parser.sico',
        'runner\sico-runner\tests\selfhost_parser.rs',
        'docs\steps\STEP-0239-m22-structured-if-ir.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0239 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_parser
if ($LASTEXITCODE -ne 0) { throw 'STEP-0239 structured if lowering differential failed' }

Write-Output 'STEP_0239_OK positive=107 control_flow=if-else forms=set-arms+return-arms blocks=branch+jump+unreachable verifier=independent equivalence=byte-exact runner=real status=partial-S4'
