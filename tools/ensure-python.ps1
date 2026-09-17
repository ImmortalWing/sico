# M19 support (STEP-0196): the Windows console-control fixtures run a
# stdlib-only Python script (runner/sico-runner/tests/runner.rs). A fresh
# host without `python` on PATH failed those tests (registered in
# STEP-0195 §5, corrected in STEP-0196). This script mirrors
# ensure-wasmtime.ps1: reuse an existing python.exe (PATH or
# target/tooling), otherwise download the pinned embeddable CPython with
# SHA256 verification. Prints the python.exe path.
param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$version = '3.12.10'
$archiveName = "python-$version-embed-amd64.zip"
$expectedSha256 = '4acbed6dd1c744b0376e3b1cf57ce906f9dc9e95e68824584c8099a63025a3c3'
$tooling = Join-Path $root 'target/tooling'
$directory = Join-Path $tooling "python-$version"
$executable = Join-Path $directory 'python.exe'

function Test-UsablePython([string]$Candidate) {
    if (-not (Test-Path -LiteralPath $Candidate -PathType Leaf)) { return $false }
    # The WindowsApps app-execution alias is a store-installer stub that
    # writes to stderr and exits non-zero; never trust that path.
    if ($Candidate -like '*\Microsoft\WindowsApps\*') { return $false }
    $previous = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try {
        & $Candidate -c "import os, signal, subprocess, sys" 2>&1 | Out-Null
        return ($LASTEXITCODE -eq 0)
    } catch {
        return $false
    } finally {
        $ErrorActionPreference = $previous
    }
}

$fromPath = Get-Command python.exe -ErrorAction SilentlyContinue
if ($fromPath -and (Test-UsablePython $fromPath.Source)) {
    Write-Output $fromPath.Source
    exit 0
}

if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    New-Item -ItemType Directory -Force -Path $directory | Out-Null
    $archive = Join-Path $tooling $archiveName
    if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
        $url = "https://www.python.org/ftp/python/$version/$archiveName"
        Invoke-WebRequest -Uri $url -OutFile $archive
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
    if ($actual -cne $expectedSha256) {
        throw "Python archive checksum mismatch: expected=$expectedSha256 actual=$actual"
    }
    Expand-Archive -LiteralPath $archive -DestinationPath $directory -Force
}

if (-not (Test-UsablePython $executable)) {
    throw "Python executable missing or unusable after extraction: $executable"
}

Write-Output $executable
