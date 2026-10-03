<#
.SYNOPSIS
    M24 codec feasibility probe runner (measurement only).

.DESCRIPTION
    Decides empirically whether the RFC-0050 codec implementation home can
    stay a PURE package Component (zero compiler/Runtime change) instead of
    moving to a host provider. It does not move any gate and claims no
    capability; it writes results/probe-report.json and prints a verdict.

    Steps:
      1. Regenerate the independent PNG fixtures (tools/make-fixtures.py).
      2. Build the guest core module (wasm32-unknown-unknown), hash it.
      3. cargo clean + rebuild the guest, hash again -> core reproducibility.
      4. Build the host probe (codec-probe.exe).
      5. Run codec-probe -> decode corpus, encode stability, refusals.

    The toolchain is the repo-pinned 1.98.0-x86_64-pc-windows-gnu. That
    target needs MinGW binutils (as/dlltool) on PATH for windows-sys
    raw-dylib; this script locates the repo-local msys2-binutils cache the
    same way tools/run-ci.ps1 does, falling back to the sibling main
    checkout when run from an external worktree.
#>

[CmdletBinding()]
param(
    [string]$RepositoryRoot = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$probeRoot = Split-Path -Parent $PSScriptRoot
if ([string]::IsNullOrEmpty($RepositoryRoot)) {
    # tools/ -> image-codec-probe/ -> prototypes/ -> repo root
    $RepositoryRoot = Split-Path -Parent (Split-Path -Parent $probeRoot)
}
$guest = Join-Path $probeRoot 'guest'
$host_ = Join-Path $probeRoot 'host'
$fixtures = Join-Path $probeRoot 'fixtures'
$results = Join-Path $probeRoot 'results'
$coreWasm = Join-Path $guest 'target\wasm32-unknown-unknown\release\sico_image_codec_probe_guest.wasm'
$probeExe = Join-Path $host_ 'target\release\codec-probe.exe'
$report = Join-Path $results 'probe-report.json'

$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'

# Locate MinGW binutils (as.exe / dlltool.exe) required by the gnu target.
function Find-Binutils {
    $candidates = @(
        (Join-Path $RepositoryRoot 'target\tooling\msys2-binutils\mingw64\bin'),
        'E:\github\sico\target\tooling\msys2-binutils\mingw64\bin'
    )
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath (Join-Path $candidate 'dlltool.exe') -PathType Leaf) {
            return $candidate
        }
    }
    throw 'MinGW binutils (dlltool.exe) not found; provision target\tooling\msys2-binutils first.'
}

function Find-Python {
    $candidates = @(
        (Join-Path $RepositoryRoot 'target\tooling\python-3.12\python.exe'),
        'E:\github\sico\target\tooling\python-3.12\python.exe'
    )
    foreach ($candidate in $candidates) {
        if (Test-Path -LiteralPath $candidate -PathType Leaf) { return $candidate }
    }
    $cmd = Get-Command python.exe -ErrorAction SilentlyContinue
    if ($cmd) { return $cmd.Source }
    throw 'python.exe not found; needed to regenerate the independent PNG fixtures.'
}

$binutils = Find-Binutils
$python = Find-Python
$env:PATH = "$binutils;$env:PATH"

New-Item -ItemType Directory -Force -Path $results | Out-Null

Write-Host "== fixtures =="
& $python (Join-Path $probeRoot 'tools\make-fixtures.py')

function Hash([string]$path) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
}

Write-Host "== guest build 1 =="
Push-Location $guest
try {
    & $cargo build --release --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'guest build 1 failed' }
    $hash1 = Hash $coreWasm

    Write-Host "== guest clean + build 2 (reproducibility) =="
    & $cargo clean
    & $cargo build --release --target wasm32-unknown-unknown
    if ($LASTEXITCODE -ne 0) { throw 'guest build 2 failed' }
    $hash2 = Hash $coreWasm
}
finally { Pop-Location }

$coreReproducible = ($hash1 -eq $hash2)
Write-Host ("core sha256 build1={0}" -f $hash1)
Write-Host ("core sha256 build2={0}" -f $hash2)
Write-Host ("core reproducible across clean rebuild: {0}" -f $coreReproducible)

Write-Host "== host probe build =="
Push-Location $host_
try {
    & $cargo build --release
    if ($LASTEXITCODE -ne 0) { throw 'host probe build failed' }
}
finally { Pop-Location }

Write-Host "== run codec-probe =="
& $probeExe $coreWasm $fixtures $report
if ($LASTEXITCODE -ne 0) { throw 'codec-probe failed' }

$decoded = Get-Content -LiteralPath $report -Raw | ConvertFrom-Json
$verdict = $decoded.verdict
$final = if ($coreReproducible -and $verdict -eq 'package-route-feasible') {
    'package-route-feasible'
} else {
    'package-route-not-demonstrated'
}

Write-Host ''
Write-Host ("PROBE VERDICT: {0}" -f $final)
Write-Host ("  core reproducible : {0}" -f $coreReproducible)
Write-Host ("  codec-probe       : {0}" -f $verdict)
Write-Host ("  report            : {0}" -f $report)
