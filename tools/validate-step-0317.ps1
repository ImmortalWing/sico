param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$compilerTests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
$parserTests = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_parser.rs') -Raw -Encoding UTF8
$runnerManifest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\Cargo.toml') -Raw -Encoding UTF8
$runCi = Get-Content -LiteralPath (Join-Path $root 'tools\run-ci.ps1') -Raw -Encoding UTF8
$support = Join-Path $root 'runner\sico-runner\tests\support\selfhost_common.rs'
if (-not (Test-Path -LiteralPath $support)) {
    throw 'STEP-0317 shared selfhost support module missing'
}
$supportText = Get-Content -LiteralPath $support -Raw -Encoding UTF8
foreach ($marker in @(
    'SICO_SELFHOST_COMPONENT_CACHE',
    'fn cached_component(',
    'pub struct PreparedPool',
    'fn component_key('
)) {
    if (-not $supportText.Contains($marker)) {
        throw "missing STEP-0317 support marker: $marker"
    }
}
foreach ($marker in @(
    'mod selfhost_common;',
    'PreparedPool::new(default_pool_slots()',
    'cached_component("compiler.sico"'
)) {
    if (-not $compilerTests.Contains($marker)) {
        throw "missing STEP-0317 compiler-harness marker: $marker"
    }
}
if (-not $parserTests.Contains('selfhost_common::cached_component(')) {
    throw 'STEP-0317 parser-harness cache marker missing'
}
if (-not $runnerManifest.Contains('[profile.dev.package."*"]') -or -not $runnerManifest.Contains('opt-level = 1')) {
    throw 'STEP-0317 runner dev-dependency opt-level override missing'
}
if (-not $runCi.Contains('Stopwatch') -or -not $runCi.Contains('=== PASS: $name ($elapsed)')) {
    throw 'STEP-0317 run-ci per-step elapsed timing missing'
}
& (Join-Path $PSScriptRoot 'validate-step-0316.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0316 inherited validator failed' }
Write-Output 'STEP_0317_OK component-cache=source+fingerprint-keyed pool=engine-isolated-lazy dev-deps=opt1 run-ci=per-step-elapsed oracle=unchanged-byte-exact'
