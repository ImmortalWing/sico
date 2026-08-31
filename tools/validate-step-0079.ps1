param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

# 1. Workspace evidence: WIT-driven layout table, deterministic Program
#    Components, independent host lift/lower helpers and 10,000 seeded
#    buffer roundtrips with malformed-memory refusals.
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
cargo test --offline --locked -p sico-codegen-wasm -p sico-runtime
if ($LASTEXITCODE -ne 0) { throw 'STEP-0079 workspace tests failed' }

# 2. Exact-engine evidence: the Wasmtime crate needs the MSVC linker on this
#    host (the pinned GNU toolchain cannot link it, see
#    prototypes/script-profile/README.md). vcvars64 must run unredirected.
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { throw "MSVC vcvars64.bat not found: $vcvars" }
$harness = Join-Path $root 'prototypes\script-abi-roundtrip'
$cmd = "call `"$vcvars`" && set RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc&& cd /d `"$harness`" && cargo run --release --offline --locked --bin sico-script-abi-roundtrip"
$report = & cmd.exe /c $cmd
if ($LASTEXITCODE -ne 0) {
    $report | ForEach-Object { Write-Host $_ }
    throw 'STEP-0079 Wasmtime roundtrip harness failed'
}
$json = ($report | Where-Object { $_ -match '^\s*[\{\}]' -or $_ -match '^\s*\"' }) -join "`n"
$summary = $json | ConvertFrom-Json
if ($summary.seeded_roundtrips -ne 10000) { throw 'unexpected roundtrip count' }
if (-not $summary.malicious_fail_closed -or $summary.malicious_fail_closed.PSObject.Properties.Name.Count -lt 6) {
    throw 'malicious-memory fixtures did not all fail closed'
}
$reportPath = Join-Path $root 'target\evidence\step-0079-script-abi-roundtrip.json'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $reportPath) | Out-Null
$json | Set-Content -Path $reportPath -Encoding utf8

Write-Output "STEP_0079_OK runtime=wasmtime-46.0.1 roundtrips=$($summary.seeded_roundtrips) cleanup=$($summary.cleanup_oracle.one_mib_calls)x1MiB+$($summary.cleanup_oracle.four_mib_calls)x4MiB malicious=$($summary.malicious_fail_closed.PSObject.Properties.Name.Count) report=$reportPath"
