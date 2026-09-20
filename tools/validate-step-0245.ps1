$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

$checker = Get-Content -LiteralPath (Join-Path $repo 'selfhost\checker.sico') -Raw
$semantics = Get-Content -LiteralPath (Join-Path $repo 'selfhost\compiler_semantics.sico') -Raw
$tests = Get-Content -LiteralPath (Join-Path $repo 'runner\sico-runner\tests\selfhost_checker.rs') -Raw
$steps = Get-ChildItem -LiteralPath (Join-Path $repo 'docs\steps') -File

foreach ($number in 213..245) {
    $prefix = 'STEP-{0:D4}-' -f $number
    $matches = @($steps | Where-Object { $_.Name.StartsWith($prefix, [StringComparison]::Ordinal) })
    if ($matches.Count -ne 1) {
        $names = ($matches.Name | Sort-Object) -join ', '
        throw "$prefix must have exactly one M22 record; found $($matches.Count): $names"
    }
}

if (-not $checker.Contains('use compiler_semantics.semantic_diagnostic')) {
    throw 'checker must consume the integrated semantic module'
}
foreach ($code in @(
    'E2001', 'E2002', 'E2010', 'E2011', 'E2020',
    'E3001', 'E3002', 'E3003', 'E3101', 'E3102', 'E3103', 'E3104',
    'E4001', 'E4002', 'E5001', 'E5002', 'E5003',
    'E5101', 'E5102', 'E5103', 'E5104', 'E5105', 'E5201', 'E5202',
    'E6001', 'E6002', 'E7001', 'E7002', 'E8010'
)) {
    if (-not $semantics.Contains('"' + $code + '"')) {
        throw "semantic module is missing $code"
    }
    if (-not $tests.Contains('"' + $code + '"')) {
        throw "real-runner suite is missing $code"
    }
}
foreach ($forbidden in @('syntax-candidates/', '// expect:', 'source_sha256')) {
    if ($semantics.Contains($forbidden)) {
        throw "semantic implementation must not identify fixtures via $forbidden"
    }
}
if (-not $tests.Contains('(116, 34, 65, 0)')) {
    throw 'complete frozen-corpus partition assertion is missing'
}
if (-not $tests.Contains('assert_eq!(entries.len(), 215)')) {
    throw 'frozen-corpus cardinality assertion is missing'
}
if ($checker.Contains('function check_source(') -or $checker.Contains('function report_line(')) {
    throw 'legacy free-form header scanner remains'
}

Push-Location $repo
try {
    & $cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'root fmt failed' }

    & $cargo test --locked --offline -p sico-semantics
    if ($LASTEXITCODE -ne 0) { throw 'Rust semantic oracle failed' }

    & $cargo fmt --all --manifest-path .\runner\sico-runner\Cargo.toml -- --check
    if ($LASTEXITCODE -ne 0) { throw 'runner fmt failed' }

    & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test bootstrap_bundle --test selfhost_checker --test selfhost_compiler --test selfhost_declaration_parser
    if ($LASTEXITCODE -ne 0) { throw 'M22 checker/parser/compiler regression failed' }

    & $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'runner clippy failed' }

    git diff --check HEAD
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
}
finally {
    Pop-Location
}
