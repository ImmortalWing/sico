$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

$parser = Get-Content -LiteralPath (Join-Path $repo 'selfhost\parser.sico') -Raw
$tests = Get-Content -LiteralPath (Join-Path $repo 'runner\sico-runner\tests\selfhost_parser.rs') -Raw

if ($parser.Contains('ERR:E-SH-IR-STATEMENTATEMENT')) {
    throw 'misspelled statement diagnostic remains'
}

$cell = $parser.IndexOf('let ge_cell_index = general_cell_lookup')
$parameter = $parser.IndexOf('let ge_param_count = count_params', $cell)
if ($cell -lt 0 -or $parameter -lt 0 -or $cell -ge $parameter) {
    throw 'general expression lookup must resolve local cells before parameters'
}

foreach ($needle in @(
    'function shadow(x: I64) returns I64:',
    'function literal_after_let() returns I64:',
    'ERR:E-SH-IR-STATEMENT'
)) {
    if (-not $tests.Contains($needle)) {
        throw "missing STEP-0241 regression: $needle"
    }
}

Push-Location $repo
try {
    & $cargo fmt --all --manifest-path .\runner\sico-runner\Cargo.toml -- --check
    if ($LASTEXITCODE -ne 0) { throw 'runner fmt failed' }

    & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser
    if ($LASTEXITCODE -ne 0) { throw 'selfhost_parser regression failed' }

    & $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'runner clippy failed' }

    git diff --check
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
}
finally {
    Pop-Location
}
