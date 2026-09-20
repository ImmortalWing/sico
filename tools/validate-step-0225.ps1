param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

foreach ($relative in @(
        'selfhost\parser.sico',
        'runner\sico-runner\tests\selfhost_parser.rs',
        'docs\steps\STEP-0225-m22-checked-nested-call-match-ir.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0225 artifact: $relative"
    }
}

& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_parser
if ($LASTEXITCODE -ne 0) { throw 'STEP-0225 checked nested-call match IR differential failed' }

Write-Output 'STEP_0225_OK positive=44 checked_nested_calls=left+right widths=I64+U64 wrong_type=typed-refusal verifier=independent equivalence=byte-exact runner=real status=partial-S4'
