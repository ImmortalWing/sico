$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

$lexer = Get-Content -LiteralPath (Join-Path $repo 'selfhost\compiler_lexer.sico') -Raw
$checker = Get-Content -LiteralPath (Join-Path $repo 'selfhost\checker.sico') -Raw
$tests = Get-Content -LiteralPath (Join-Path $repo 'runner\sico-runner\tests\selfhost_checker.rs') -Raw
$steps = Get-ChildItem -LiteralPath (Join-Path $repo 'docs\steps') -File

foreach ($number in 213..242) {
    $prefix = 'STEP-{0:D4}-' -f $number
    $matches = @($steps | Where-Object { $_.Name.StartsWith($prefix, [StringComparison]::Ordinal) })
    if ($matches.Count -ne 1) {
        $names = ($matches.Name | Sort-Object) -join ', '
        throw "$prefix must have exactly one M22 record; found $($matches.Count): $names"
    }
}

foreach ($needle in @(
    'function token_finish(',
    'function token_kind(',
    'function has_error('
)) {
    if (-not $lexer.Contains($needle)) {
        throw "shared lexer core is missing: $needle"
    }
}

if (-not $checker.Contains('use compiler_lexer.has_error')) {
    throw 'checker must consume the integrated lexer module'
}
if (-not $checker.Contains('message: "LEXICAL"')) {
    throw 'checker must preserve the stable LEXICAL identity'
}
foreach ($needle in @('entries.len(), 215', '(lexical, non_lexical), (116, 99)')) {
    if (-not $tests.Contains($needle)) {
        throw "missing full-corpus checker assertion: $needle"
    }
}

Push-Location $repo
try {
    & $cargo fmt --all --manifest-path .\runner\sico-runner\Cargo.toml -- --check
    if ($LASTEXITCODE -ne 0) { throw 'runner fmt failed' }

    & $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test bootstrap_bundle --test selfhost_checker --test selfhost_compiler
    if ($LASTEXITCODE -ne 0) { throw 'M22 corpus/checker/compiler regression failed' }

    & $cargo clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings
    if ($LASTEXITCODE -ne 0) { throw 'runner clippy failed' }

    git diff --check HEAD
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
}
finally {
    Pop-Location
}
