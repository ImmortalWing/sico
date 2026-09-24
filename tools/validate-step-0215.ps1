param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

# STEP-0278 (W1 consolidation, restart-assessment §5 item 1): the legacy
# per-unit lexer/tokens pair was retired in favor of the integrated
# compiler_lexer/compiler_parser chain. Retirement invariants: the legacy
# sources and their per-unit tests must be gone.
foreach ($relative in @(
        'selfhost\tokens.sico',
        'runner\sico-runner\tests\selfhost_tokens.rs'
    )) {
    if (Test-Path -LiteralPath (Join-Path $root $relative)) {
        throw "retired STEP-0215 artifact still present: $relative"
    }
}

# History anchors stay frozen (corpus bytes are sha-pinned by selfhost_checker).
foreach ($relative in @(
        'selfhost\corpus-v0.json',
        'docs\steps\STEP-0215-m22-lossless-lexer-corpus.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0215 history anchor: $relative"
    }
}

# The 215-source frozen corpus now runs through the integrated chain.
& $cargo test --locked --offline --manifest-path (Join-Path $root 'runner\sico-runner\Cargo.toml') --test selfhost_checker
if ($LASTEXITCODE -ne 0) { throw 'STEP-0215 retirement: integrated-chain corpus differential failed' }

Write-Output 'STEP_0215_OK corpus=215 lexer=integrated-retired equivalence=byte-exact runner=real retired-by=STEP-0278'
