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

$core = Join-Path $root 'target/evidence/step-0078-general.core.wasm'
& $node (Join-Path $root 'tools/validate-core-wasm-runtime.mjs') --general-out $core
if ($LASTEXITCODE -ne 0) { throw 'general Core Wasm Node oracle failed' }

$cases = @(
    @{ Invoke = 'call_identity'; Arguments = @('123'); Expected = '123' },
    @{ Invoke = 'jump_chain'; Arguments = @('-9'); Expected = '-9' },
    @{ Invoke = 'bool_match'; Arguments = @('1'); Expected = '41' },
    @{ Invoke = 'bool_match'; Arguments = @('0'); Expected = '42' },
    @{ Invoke = 'record_project'; Arguments = @(); Expected = '22' },
    @{ Invoke = 'variant_match'; Arguments = @(); Expected = '1' },
    @{ Invoke = 'cycle_gate'; Arguments = @('1'); Expected = '7' }
)
foreach ($case in $cases) {
    $actual = ((& $wasmtime run --invoke $case.Invoke $core @($case.Arguments)) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0 -or $actual -cne $case.Expected) {
        throw "Wasmtime mismatch for $($case.Invoke): expected=$($case.Expected) actual=$actual"
    }
}

$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
$trapOutput = ((& $wasmtime run -W fuel=10000 --invoke cycle_gate $core 0 2>&1) -join "`n")
$trapExit = $LASTEXITCODE
$ErrorActionPreference = $previousPreference
if ($trapExit -eq 0 -or $trapOutput -notmatch 'all fuel consumed by WebAssembly') {
    throw "cycle back-edge did not exhaust bounded fuel: exit=$trapExit output=$trapOutput"
}

Write-Output "STEP_0078_OK runtime=$version cases=$($cases.Count) direct_call=pass cfg=jump,branch,match,back-edge internal_data=record,variant loop_policy=fuel-trap"
