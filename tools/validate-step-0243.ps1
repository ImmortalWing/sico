$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

$checker = Get-Content -LiteralPath (Join-Path $repo 'selfhost\checker.sico') -Raw
$parser = Get-Content -LiteralPath (Join-Path $repo 'selfhost\compiler_parser.sico') -Raw
$tests = Get-Content -LiteralPath (Join-Path $repo 'runner\sico-runner\tests\selfhost_checker.rs') -Raw
$steps = Get-ChildItem -LiteralPath (Join-Path $repo 'docs\steps') -File

foreach ($number in 213..243) {
    $prefix = 'STEP-{0:D4}-' -f $number
    $matches = @($steps | Where-Object { $_.Name.StartsWith($prefix, [StringComparison]::Ordinal) })
    if ($matches.Count -ne 1) {
        $names = ($matches.Name | Sort-Object) -join ', '
        throw "$prefix must have exactly one M22 record; found $($matches.Count): $names"
    }
}

if (-not $checker.Contains('use compiler_parser.syntax_diagnostic')) {
    throw 'checker must consume the integrated syntax diagnostic path'
}
foreach ($number in 1001..1016) {
    $code = 'E{0}' -f $number
    if (-not $parser.Contains('"' + $code + '"')) {
        throw "integrated parser is missing $code"
    }
    if (-not $tests.Contains('"' + $code + '"')) {
        throw "real-runner suite is missing $code"
    }
}

Push-Location $repo
try {
    & $cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw 'root fmt failed' }

    & $cargo test --locked --offline -p sico-parser
    if ($LASTEXITCODE -ne 0) { throw 'Rust parser oracle failed' }

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
