param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$version = '46.0.1'
$archiveName = "wasmtime-v$version-x86_64-mingw.zip"
$expectedSha256 = '7e807f480aaaf65ab6bfb765a0be05af0da235058d4e33678a68b2288f13796c'
$tooling = Join-Path $root 'target/tooling'
$archive = Join-Path $tooling $archiveName
$directory = Join-Path $tooling "wasmtime-v$version-x86_64-mingw"
$executable = Join-Path $directory 'wasmtime.exe'

if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    New-Item -ItemType Directory -Force -Path $tooling | Out-Null
    if (-not (Test-Path -LiteralPath $archive -PathType Leaf)) {
        $url = "https://github.com/bytecodealliance/wasmtime/releases/download/v$version/$archiveName"
        Invoke-WebRequest -Uri $url -OutFile $archive
    }
    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash.ToLowerInvariant()
    if ($actual -cne $expectedSha256) {
        throw "Wasmtime archive checksum mismatch: expected=$expectedSha256 actual=$actual"
    }
    Expand-Archive -LiteralPath $archive -DestinationPath $tooling -Force
}

if (-not (Test-Path -LiteralPath $executable -PathType Leaf)) {
    throw "Wasmtime executable missing after extraction: $executable"
}

Write-Output $executable
