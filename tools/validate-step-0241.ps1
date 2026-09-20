$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

$parser = Get-Content -LiteralPath (Join-Path $repo 'selfhost\parser.sico') -Raw
$tests = Get-Content -LiteralPath (Join-Path $repo 'runner\sico-runner\tests\selfhost_parser.rs') -Raw
$steps = Get-ChildItem -LiteralPath (Join-Path $repo 'docs\steps') -File

foreach ($number in 213..241) {
    $prefix = 'STEP-{0:D4}-' -f $number
    $matches = @($steps | Where-Object { $_.Name.StartsWith($prefix, [StringComparison]::Ordinal) })
    if ($matches.Count -ne 1) {
        $names = ($matches.Name | Sort-Object) -join ', '
        throw "$prefix must have exactly one M22 record; found $($matches.Count): $names"
    }
}

if ($parser.Contains('ERR:E-SH-IR-STATEMENTATEMENT')) {
    throw 'misspelled statement diagnostic remains'
}

if (-not $parser.Contains('return "ERR:E-SH-IR-CELL-SHADOW"')) {
    throw 'advanced cell frontend must fail closed on parameter shadowing'
}

foreach ($needle in @(
    'function shadow(x: I64) returns I64:',
    'ERR:E-SH-IR-CELL-SHADOW'
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

    git diff --check HEAD
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
}
finally {
    Pop-Location
}
