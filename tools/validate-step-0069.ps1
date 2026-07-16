param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$manifest = Read-RepoFile 'pilots/third-party-component/pilot.json' | ConvertFrom-Json
$cases = Read-RepoFile 'tests/ecosystem/m7-pilot-cases.json' | ConvertFrom-Json
$performance = Read-RepoFile 'tests/performance/m7-pilot-windows-debug.json' | ConvertFrom-Json
$testSource = Read-RepoFile 'pilots/third-party-component/tests/pilot.rs'
$step = Read-RepoFile 'docs/steps/STEP-0069-third-party-pilot-m7-exit.md'
$audit = Read-RepoFile 'docs/reports/m7-exit-audit.md'

if ($manifest.schema -ne 'sico.third-party-pilot.v0' -or $manifest.evidence_level -ne 'repository-authored-clean-room') { throw 'pilot evidence label drifted' }
if (@($manifest.local_stages).Count -ne 11) { throw 'pilot must preserve eleven local stages' }
if (@($manifest.external_evidence.PSObject.Properties).Count -ne 7 -or @($manifest.external_evidence.PSObject.Properties.Value | Where-Object { $_ -eq 'complete' }).Count -ne 0) { throw 'external evidence must preserve seven incomplete gates' }
if ($cases.schema -ne 'sico.ecosystem.m7-pilot-cases.v0' -or @($cases.cases).Count -ne 24 -or @($cases.cases.id | Sort-Object -Unique).Count -ne 24) { throw 'pilot corpus must contain 24 unique cases' }
foreach ($area in 'editor','ai','ui','build','package','publisher','registry','host','runtime','security','evidence') {
    if (@($cases.cases | Where-Object area -eq $area).Count -eq 0) { throw "pilot case area missing: $area" }
}
if ($performance.schema -ne 'sico.m7.pilot-release-drill.v0' -or -not $performance.runtime_verified -or $performance.result -ne 42 -or $performance.security_rejections -ne 4 -or $performance.sla -ne 'not-established') { throw 'pilot performance evidence incomplete' }
if (([regex]::Matches($testSource, '(?m)^#\[test\]\r?$')).Count -ne 4) { throw 'pilot test count drifted' }
foreach ($needle in 'include_str!("../app-v1.sico")','include_str!("../app-v2.sico")','run_authorized_package','security_rejections, 4','PILOT_RELEASE_METRICS') {
    if (-not $testSource.Contains($needle)) { throw "pilot invariant missing: $needle" }
}
if ($step -notmatch '(?m)^> - status: complete-local / blocked-external-evidence\r?$' -or $audit -notmatch '(?m)^> - M7/product exit: NO-GO\r?$') { throw 'STEP-0069 audit disposition drifted' }

$toolchain = Join-Path $env:USERPROFILE '.rustup\toolchains\stable-x86_64-pc-windows-gnu\bin'
$cargo = Join-Path $toolchain 'cargo.exe'
if (-not (Test-Path -LiteralPath $cargo -PathType Leaf)) { throw "stable cargo missing: $cargo" }
$env:RUSTC = Join-Path $toolchain 'rustc.exe'
$env:RUSTDOC = Join-Path $toolchain 'rustdoc.exe'
$env:RUSTFMT = Join-Path $toolchain 'rustfmt.exe'
$env:__COMPAT_LAYER = 'RunAsInvoker'
$env:SICO_TEST_WASMTIME = & (Join-Path $root 'tools\ensure-wasmtime.ps1') -RepositoryRoot $root
$output = (& $cargo test --locked --offline -p sico-third-party-pilot --quiet -- --nocapture 2>&1 | Out-String)
if ($LASTEXITCODE -ne 0) { throw "pilot tests failed:`n$output" }
foreach ($needle in '4 passed','releases=2','result=42','runtime=true','security_rejections=4') {
    if (-not $output.Contains($needle)) { throw "pilot runtime output missing: $needle" }
}

Write-Output 'STEP_0069_LOCAL_OK tests=4 stages=11 releases=2 result=42 runtime=true security_rejections=4 cases=24 third_party=external-gated production=external-gated live_model=external-gated mobile=external-gated linux_runner=external-gated project=blocked-external-evidence'
