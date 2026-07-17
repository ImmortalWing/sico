param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$node = (Get-Command node -ErrorAction Stop).Source
$wasmtime = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
$version = (& $wasmtime --version).Trim()
if ($LASTEXITCODE -ne 0 -or $version -cne 'wasmtime 46.0.1 (823d1b8f2 2026-06-24)') {
    throw "unexpected Wasmtime runtime: $version"
}

$component = Join-Path $root 'target/evidence/step-0077-fixed-width.component.wasm'
& $node (Join-Path $root 'tools/validate-core-wasm-runtime.mjs') --component-out $component
if ($LASTEXITCODE -ne 0) { throw 'Core Wasm fixed-width oracle failed' }

$cases = @(
    @{ Invoke = 'i64-checked-add(40, 2)'; Expected = 'ok(42)' },
    @{ Invoke = 'i64-checked-add(9223372036854775807, 1)'; Expected = 'err(overflow)' },
    @{ Invoke = 'i64-checked-add(-9223372036854775808, -1)'; Expected = 'err(underflow)' },
    @{ Invoke = 'i64-checked-sub(9223372036854775807, -1)'; Expected = 'err(overflow)' },
    @{ Invoke = 'i64-checked-sub(-9223372036854775808, 1)'; Expected = 'err(underflow)' },
    @{ Invoke = 'u64-checked-add(18446744073709551615, 1)'; Expected = 'err(overflow)' },
    @{ Invoke = 'u64-checked-sub(0, 1)'; Expected = 'err(underflow)' },
    @{ Invoke = 'i64-less-than(-1, 0)'; Expected = 'true' },
    @{ Invoke = 'u64-less-than(18446744073709551615, 0)'; Expected = 'false' },
    @{ Invoke = 'i64-equal(7, 7)'; Expected = 'true' },
    @{ Invoke = 'u64-equal(7, 8)'; Expected = 'false' }
)
foreach ($case in $cases) {
    $actual = ((& $wasmtime run --invoke $case.Invoke $component) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0 -or $actual -cne $case.Expected) {
        throw "Wasmtime mismatch for $($case.Invoke): expected=$($case.Expected) actual=$actual"
    }
}

Write-Output "STEP_0077_OK runtime=$version component_cases=$($cases.Count) core_properties=2048x8 typed_failures=overflow,underflow"
