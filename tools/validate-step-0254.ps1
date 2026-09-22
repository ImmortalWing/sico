param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$compilerTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8

foreach ($marker in @(
    'CancelToken, FsGrants, NetGrants, PreparedProgram, RunOutcome, Runner, RunnerLimits, ScriptInput',
    'Reuse native compilation, not guest execution state.',
    'allocates a fresh Store, resources and fuel for every fixture',
    'runs because timeout watchdogs advance the shared Engine',
    'static PREPARED: std::sync::OnceLock<std::sync::Mutex<PreparedProgram>>',
    'The same prepared compiler must remain usable after a domain refusal.',
    'valid input after refusal must compile in a fresh Store'
)) {
    if (-not $compilerTest.Contains($marker)) { throw "missing STEP-0254 marker: $marker" }
}

# Exactly two native compilations remain: the shared prepared program and the
# dedicated lexer self-check. A per-fixture recompile would raise this count.
$preparations = [regex]::Matches($compilerTest, 'prepare_program_with_net').Count
if ($preparations -ne 2) { throw "expected 2 prepare_program_with_net sites, found $preparations" }

if (-not (Test-Path -LiteralPath (Join-Path $root 'docs\steps\STEP-0254-m22-prepared-compiler-test-reuse.md'))) {
    throw 'missing STEP-0254 record document'
}
foreach ($doc in @(
    @{ Path = 'docs\steps\README.md'; Marker = 'STEP-0254-m22-prepared-compiler-test-reuse.md' },
    @{ Path = 'docs\ROADMAP.md'; Marker = 'STEP-0254' },
    @{ Path = 'docs\STATUS.md'; Marker = 'STEP-0254' },
    @{ Path = 'docs\plans\M22-compiler-self-host.md'; Marker = 'STEP-0254' }
)) {
    $text = Get-Content -LiteralPath (Join-Path $root $doc.Path) -Raw -Encoding UTF8
    if (-not $text.Contains($doc.Marker)) { throw "missing STEP-0254 reference in $($doc.Path)" }
}

Push-Location $root
try {
    # This host's runner artifacts are built with the pinned MSVC toolchain
    # (cc-rs auto-discovers the VS 2022 cl.exe, as recorded in the cached
    # ring build output). The GNU flow additionally requires a real gcc for
    # `ring` and dlltool from target/tooling/msys2-binutils, which are not
    # present on this host right now; switching toolchains forces a full
    # rebuild that cannot complete. Pin the toolchain that matches the cache.
    $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-msvc'
    $clock = [Diagnostics.Stopwatch]::StartNew()
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host compiler differential failed' }
    $clock.Stop()
    Write-Output ("serial differential wall-clock: {0:n2}s (STEP-0253 baseline 801.46s; not an exit gate)" -f $clock.Elapsed.TotalSeconds)

    git diff --check
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
}
finally {
    Pop-Location
}

Write-Output 'STEP_0254_OK harness=prepared-program-reuse stores=fresh-per-run refusal-recovery=pinned support-boundary=unchanged'
