<#
.SYNOPSIS
    ADR-0019 §4 — package-core byte-reproducibility validator.

.DESCRIPTION
    For each Rust-compiled package core registered below, rebuilds the
    out-of-workspace guest crate from a clean tree (pinned toolchain, fixed
    release profile, --offline --locked) and asserts the rebuilt core is
    BYTE-IDENTICAL to the committed digest-pinned artifact under
    crates/sico-codegen-wasm/artifacts. A codec/package STEP is not green
    unless this passes on the evidence host.

    A crate or toolchain bump that changes bytes is a NEW package version and
    must be re-pinned deliberately (update the committed .wasm + .sha256), not
    absorbed silently.

    The toolchain is the repo-pinned 1.98.0-x86_64-pc-windows-gnu. The gnu
    target needs MinGW binutils on PATH for windows-sys raw-dylib; this script
    locates the repo-local msys2-binutils cache the same way tools/run-ci.ps1
    does, falling back to the sibling main checkout when run from an external
    worktree.

.PARAMETER RepositoryRoot
    Repo root. Defaults to the parent of this script's tools/ directory.

.EXAMPLE
    powershell -File tools/validate-package-core.ps1
#>

[CmdletBinding()]
param(
    [string]$RepositoryRoot = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([string]::IsNullOrEmpty($RepositoryRoot)) {
    $RepositoryRoot = Split-Path -Parent $PSScriptRoot
}

# Registry of Rust-compiled package cores (ADR-0019 class (b)). Add a row per
# new committed core (e.g. image-codec JPEG in M24 #3).
$cores = @(
    [pscustomobject]@{
        Name     = 'image-codec'
        Guest    = 'guests/image-codec'
        CoreName = 'sico_image_codec_guest.wasm'
        Artifact = 'crates/sico-codegen-wasm/artifacts/image-codec-core.wasm'
    }
    [pscustomobject]@{
        Name     = 'image-match'
        Guest    = 'guests/image-match'
        CoreName = 'sico_image_match_guest.wasm'
        Artifact = 'crates/sico-codegen-wasm/artifacts/image-match-core.wasm'
    }
    [pscustomobject]@{
        Name     = 'image-grid'
        Guest    = 'guests/image-grid'
        CoreName = 'sico_image_grid_guest.wasm'
        Artifact = 'crates/sico-codegen-wasm/artifacts/image-grid-core.wasm'
    }
    [pscustomobject]@{
        Name     = 'image-contour'
        Guest    = 'guests/image-contour'
        CoreName = 'sico_image_contour_guest.wasm'
        Artifact = 'crates/sico-codegen-wasm/artifacts/image-contour-core.wasm'
    }
)

$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'

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

function Hash([string]$path) {
    (Get-FileHash -Algorithm SHA256 -LiteralPath $path).Hash.ToLowerInvariant()
}

$binutils = Find-Binutils
$env:PATH = "$binutils;$env:PATH"

$allOk = $true
foreach ($core in $cores) {
    Write-Host ("== {0}: reproducibility ==" -f $core.Name)
    $guest = Join-Path $RepositoryRoot $core.Guest
    $artifact = Join-Path $RepositoryRoot $core.Artifact
    $shaFile = "$artifact.sha256"
    if (-not (Test-Path -LiteralPath $artifact -PathType Leaf)) {
        Write-Host ("  MISSING committed artifact: {0}" -f $core.Artifact)
        $allOk = $false
        continue
    }
    $committedHash = Hash $artifact
    $recorded = (Get-Content -LiteralPath $shaFile -Raw).Trim().Split(' ')[0].ToLowerInvariant()
    if ($recorded -ne $committedHash) {
        Write-Host ("  RECORDED sha256 {0} != artifact {1}" -f $recorded, $committedHash)
        $allOk = $false
    }

    $built = Join-Path $guest ("target\wasm32-unknown-unknown\release\" + $core.CoreName)
    Push-Location $guest
    try {
        & $cargo clean
        if ($LASTEXITCODE -ne 0) { throw ("{0}: cargo clean failed" -f $core.Name) }
        & $cargo build --release --target wasm32-unknown-unknown --offline --locked
        if ($LASTEXITCODE -ne 0) { throw ("{0}: guest rebuild failed" -f $core.Name) }
    }
    finally { Pop-Location }

    $rebuiltHash = Hash $built
    $reproducible = ($rebuiltHash -eq $committedHash)
    Write-Host ("  committed sha256 : {0}" -f $committedHash)
    Write-Host ("  rebuilt   sha256 : {0}" -f $rebuiltHash)
    Write-Host ("  byte-identical   : {0}" -f $reproducible)
    if (-not $reproducible) { $allOk = $false }
}

Write-Host ''
if ($allOk) {
    Write-Host 'PACKAGE-CORE VERDICT: reproducible'
    exit 0
}
else {
    Write-Host 'PACKAGE-CORE VERDICT: NOT-reproducible'
    exit 1
}
