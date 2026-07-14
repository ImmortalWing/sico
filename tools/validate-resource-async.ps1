[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$manifest = Join-Path $root 'prototypes/resource-async/Cargo.toml'
$crateRoot = Join-Path $root 'prototypes/resource-async'
$deps = Join-Path $crateRoot 'target/debug/deps'

& cargo build --quiet --manifest-path $manifest --lib
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

$library = Get-ChildItem -LiteralPath $deps -Filter 'libsico_resource_async_prototype-*.rlib' |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $library) {
    throw 'resource-async rlib not found'
}

$accepted = 0
$rejected = 0
foreach ($case in Get-ChildItem -LiteralPath (Join-Path $crateRoot 'tests/ui') -Filter '*.rs' | Sort-Object Name) {
    $source = Get-Content -Raw -LiteralPath $case.FullName
    $match = [regex]::Match($source, 'expect-code:\s*(E\d{4})')
    if (-not $match.Success) {
        throw "missing expect-code in $($case.Name)"
    }
    $expected = $match.Groups[1].Value
    $previousPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    $output = & rustc --edition=2024 --crate-type=lib --emit=metadata `
        --extern "sico_resource_async_prototype=$($library.FullName)" `
        -L "dependency=$deps" $case.FullName 2>&1 | Out-String
    $compileExit = $LASTEXITCODE
    $ErrorActionPreference = $previousPreference
    if ($compileExit -eq 0) {
        $accepted++
        throw "compile-fail case unexpectedly accepted: $($case.Name)"
    }
    if ($output -notmatch [regex]::Escape($expected)) {
        throw "compile-fail case $($case.Name) missed $expected`n$output"
    }
    $rejected++
}

Write-Output "RESOURCE_ASYNC_COMPILE_FAIL_OK rejected=$rejected accepted=$accepted"
