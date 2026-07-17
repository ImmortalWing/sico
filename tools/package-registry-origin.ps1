[CmdletBinding()]
param(
    [string]$OutputRoot
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function New-DeterministicZip {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SourceRoot,
        [Parameter(Mandatory = $true)]
        [string]$Destination
    )

    Add-Type -AssemblyName System.IO.Compression
    if (Test-Path -LiteralPath $Destination) {
        Remove-Item -LiteralPath $Destination -Force
    }
    $fixedTime = [DateTimeOffset]::new(1980, 1, 1, 0, 0, 0, [TimeSpan]::Zero)
    $stream = [IO.File]::Open($Destination, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
    $archive = $null
    try {
        $archive = [IO.Compression.ZipArchive]::new($stream, [IO.Compression.ZipArchiveMode]::Create, $true)
        $directoryEntry = $archive.CreateEntry('data/')
        $directoryEntry.LastWriteTime = $fixedTime
        foreach ($file in @(Get-ChildItem -LiteralPath $SourceRoot -Recurse -File | Sort-Object FullName)) {
            $relative = $file.FullName.Substring($SourceRoot.TrimEnd('\').Length).TrimStart('\').Replace('\', '/')
            $entry = $archive.CreateEntry($relative, [IO.Compression.CompressionLevel]::Optimal)
            $entry.LastWriteTime = $fixedTime
            $input = [IO.File]::OpenRead($file.FullName)
            $output = $entry.Open()
            try {
                $input.CopyTo($output)
            } finally {
                $output.Dispose()
                $input.Dispose()
            }
        }
    } finally {
        if ($archive) { $archive.Dispose() }
        $stream.Dispose()
    }
}

$repositoryRoot = (Resolve-Path (Split-Path -Parent $PSScriptRoot)).Path
$cargo = (Resolve-Path (Join-Path $HOME '.cargo/bin/cargo.exe')).Path
$metadata = ((& $cargo metadata --format-version 1 --locked --offline) -join "`n") | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) { throw 'cargo metadata failed' }
$package = @($metadata.packages | Where-Object name -eq 'sico-registry-server')
if ($package.Count -ne 1) { throw 'sico-registry-server package is missing' }
$version = [string]$package[0].version
$targetTriple = 'x86_64-pc-windows-gnu'
$archiveName = "sico-registry-origin-v$version-$targetTriple.zip"

if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $repositoryRoot "dist/registry-origin/v$version"
} elseif (-not [IO.Path]::IsPathRooted($OutputRoot)) {
    $OutputRoot = Join-Path $repositoryRoot $OutputRoot
}
$OutputRoot = [IO.Path]::GetFullPath($OutputRoot)

$targetRoot = [IO.Path]::GetFullPath((Join-Path $repositoryRoot 'target'))
$staging = [IO.Path]::GetFullPath((Join-Path $targetRoot 'registry-origin-package'))
$targetPrefix = $targetRoot.TrimEnd('\') + '\'
if (-not $staging.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw "Unsafe staging path: $staging"
}
if (Test-Path -LiteralPath $staging) {
    Remove-Item -LiteralPath $staging -Recurse -Force
}
New-Item -ItemType Directory -Path $staging | Out-Null
New-Item -ItemType Directory -Path (Join-Path $staging 'data') | Out-Null
New-Item -ItemType Directory -Path $OutputRoot -Force | Out-Null

try {
    & $cargo build --locked --offline --release -p sico-registry-server
    if ($LASTEXITCODE -ne 0) { throw 'release origin build failed' }
    $binary = Join-Path $repositoryRoot 'target/release/sico-registry.exe'
    if (-not (Test-Path -LiteralPath $binary -PathType Leaf)) {
        throw "origin binary is missing: $binary"
    }
    Copy-Item -LiteralPath $binary -Destination $staging
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'LICENSE') -Destination $staging
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'deploy/registry-origin/README.md') -Destination $staging

    & (Join-Path $staging 'sico-registry.exe') check --root (Join-Path $staging 'data')
    if ($LASTEXITCODE -ne 0) { throw 'packaged origin readiness check failed' }

    $start = @(
        '@echo off',
        'setlocal',
        '"%~dp0sico-registry.exe" serve --root "%~dp0data" --listen 127.0.0.1:8787',
        'exit /b %ERRORLEVEL%'
    ) -join "`r`n"
    [IO.File]::WriteAllText((Join-Path $staging 'start-registry.cmd'), "$start`r`n", [Text.Encoding]::ASCII)

    $archive = Join-Path $OutputRoot $archiveName
    New-DeterministicZip -SourceRoot $staging -Destination $archive
    $sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    $manifest = [ordered]@{
        schema = 'sico.registry-origin.bundle.v0'
        version = $version
        target = $targetTriple
        archive = $archiveName
        bytes = (Get-Item -LiteralPath $archive).Length
        sha256 = $sha256
        transport = 'read-only-http-origin'
        publicProductionEvidence = $false
    }
    $utf8 = [Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText(
        (Join-Path $OutputRoot 'registry-origin-manifest.json'),
        (($manifest | ConvertTo-Json) + "`n"),
        $utf8
    )
    Write-Host "REGISTRY_ORIGIN_BUNDLE_OK version=$version sha256=$sha256"
    Write-Host "Output: $archive"
} finally {
    if (Test-Path -LiteralPath $staging) {
        Remove-Item -LiteralPath $staging -Recurse -Force
    }
}
