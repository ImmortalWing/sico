param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

# STEP-0278 (W1 consolidation, restart-assessment §5 item 1): the legacy
# declaration_parser was retired in favor of the integrated compiler_parser
# chain. Retirement invariants: legacy source and per-unit test must be gone.
foreach ($relative in @(
        'selfhost\declaration_parser.sico',
        'runner\sico-runner\tests\selfhost_declaration_parser.rs'
    )) {
    if (Test-Path -LiteralPath (Join-Path $root $relative)) {
        throw "retired STEP-0219 artifact still present: $relative"
    }
}

foreach ($relative in @(
        'selfhost\corpus-v0.json',
        'docs\steps\STEP-0219-m22-declaration-metadata.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0219 history anchor: $relative"
    }
}

# The declaration-metadata differential now runs through the integrated chain.
& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_checker
if ($LASTEXITCODE -ne 0) { throw 'STEP-0219 retirement: integrated-chain metadata differential failed' }

Write-Output 'STEP_0219_OK shape=integrated-retired declaration_metadata=byte-exact runner=real retired-by=STEP-0278'
