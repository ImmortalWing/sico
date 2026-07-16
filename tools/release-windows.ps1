[CmdletBinding()]
param(
    [string]$Version,
    [string]$RuntimePath,
    [string]$OutputRoot,
    [switch]$AllowDirty,
    [switch]$SkipQualityGates
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Resolve-NativeTool {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Name,
        [string]$PreferredPath
    )

    if ($PreferredPath -and (Test-Path -LiteralPath $PreferredPath -PathType Leaf)) {
        return (Resolve-Path -LiteralPath $PreferredPath).Path
    }
    return (Get-Command $Name -ErrorAction Stop).Source
}

function Invoke-Checked {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Executable,
        [Parameter(Mandatory = $true)]
        [AllowEmptyCollection()]
        [string[]]$Arguments,
        [Parameter(Mandatory = $true)]
        [string]$Description
    )

    Write-Host "==> $Description"
    & $Executable @Arguments
    $exitCode = $LASTEXITCODE
    if ($exitCode -ne 0) {
        throw "$Description failed with exit code $exitCode"
    }
}

function Assert-SafeChildPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Parent,
        [Parameter(Mandatory = $true)]
        [string]$Child
    )

    $parentPath = [IO.Path]::GetFullPath($Parent).TrimEnd('\') + '\'
    $childPath = [IO.Path]::GetFullPath($Child)
    if (-not $childPath.StartsWith($parentPath, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Path must stay under '$Parent': $childPath"
    }
    if ($childPath.TrimEnd('\') -eq $parentPath.TrimEnd('\')) {
        throw "Refusing to operate on parent directory itself: $childPath"
    }
}

function Reset-Directory {
    param(
        [Parameter(Mandatory = $true)]
        [string]$SafeParent,
        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    Assert-SafeChildPath -Parent $SafeParent -Child $Path
    if (Test-Path -LiteralPath $Path) {
        Remove-Item -LiteralPath $Path -Recurse -Force
    }
    New-Item -ItemType Directory -Path $Path | Out-Null
}

function Get-AssetRecord {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Path,
        [Parameter(Mandatory = $true)]
        [string]$Role
    )

    $file = Get-Item -LiteralPath $Path
    return [ordered]@{
        name = $file.Name
        role = $Role
        bytes = $file.Length
        sha256 = (Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash.ToLowerInvariant()
    }
}

$repositoryRoot = (Resolve-Path (Split-Path -Parent $PSScriptRoot)).Path
$cargo = Resolve-NativeTool -Name 'cargo' -PreferredPath (Join-Path $HOME '.cargo/bin/cargo.exe')
$rustc = Resolve-NativeTool -Name 'rustc' -PreferredPath (Join-Path $HOME '.cargo/bin/rustc.exe')
$git = Resolve-NativeTool -Name 'git'

Push-Location $repositoryRoot
try {
    $status = @(& $git status --porcelain=v1 --untracked-files=all)
    if ($LASTEXITCODE -ne 0) {
        throw 'git status failed'
    }
    $dirty = $status.Count -ne 0
    if ($dirty -and -not $AllowDirty) {
        throw "Release requires a clean worktree. Commit or stash these changes first:`n$($status -join "`n")"
    }

    $metadataText = (& $cargo metadata --format-version 1 --locked --offline) -join "`n"
    if ($LASTEXITCODE -ne 0) {
        throw 'cargo metadata failed'
    }
    $metadata = $metadataText | ConvertFrom-Json
    $releasePackageNames = @('sico-cli', 'sico-app-cli', 'sico-language-server', 'sico-ai-tools')
    $releasePackages = @($metadata.packages | Where-Object { $releasePackageNames -contains $_.name })
    if ($releasePackages.Count -ne $releasePackageNames.Count) {
        throw 'Cargo metadata is missing one or more release packages'
    }
    $workspaceVersions = @($releasePackages.version | Sort-Object -Unique)
    if ($workspaceVersions.Count -ne 1) {
        throw "Release package versions disagree: $($workspaceVersions -join ', ')"
    }
    $workspaceVersion = [string]$workspaceVersions[0]

    if ([string]::IsNullOrWhiteSpace($Version)) {
        $Version = $workspaceVersion
    }
    $Version = $Version.Trim()
    if ($Version.StartsWith('v', [StringComparison]::OrdinalIgnoreCase)) {
        $Version = $Version.Substring(1)
    }
    if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') {
        throw "Version is not canonical SemVer: $Version"
    }
    if ($Version -cne $workspaceVersion) {
        throw "Requested version '$Version' does not match Cargo workspace version '$workspaceVersion'"
    }

    $targetTriple = 'x86_64-pc-windows-gnu'
    $rustcVersion = @(& $rustc -vV)
    if ($LASTEXITCODE -ne 0) {
        throw 'rustc -vV failed'
    }
    if (-not ($rustcVersion -match "^host: $([regex]::Escape($targetTriple))$")) {
        throw "Windows release must run with Rust host $targetTriple"
    }

    $commit = ((& $git rev-parse HEAD) -join '').Trim()
    if ($LASTEXITCODE -ne 0 -or $commit -notmatch '^[0-9a-f]{40}$') {
        throw 'Unable to resolve the release commit'
    }
    $commitDateText = ((& $git show -s --format=%cI HEAD) -join '').Trim()
    $created = [DateTimeOffset]::Parse($commitDateText).UtcDateTime.ToString('yyyy-MM-ddTHH:mm:ssZ')
    $tag = "v$Version"
    $channel = if ($Version.Contains('-')) { 'preview' } else { 'stable' }

    if (-not $SkipQualityGates) {
        Invoke-Checked -Executable $cargo -Arguments @('fmt', '--all', '--', '--check') -Description 'cargo fmt'
        Invoke-Checked -Executable $cargo -Arguments @('clippy', '--locked', '--offline', '--workspace', '--all-targets', '--all-features', '--', '-D', 'warnings') -Description 'cargo clippy'
        $previousCompatLayer = $env:__COMPAT_LAYER
        try {
            $env:__COMPAT_LAYER = 'RunAsInvoker'
            Invoke-Checked -Executable $cargo -Arguments @('test', '--locked', '--offline', '--workspace', '--all-targets', '--all-features') -Description 'cargo test'
        } finally {
            $env:__COMPAT_LAYER = $previousCompatLayer
        }
        Write-Host '==> module boundary validation'
        & (Join-Path $PSScriptRoot 'validate-module-boundaries.ps1')
    }

    Invoke-Checked -Executable $cargo -Arguments @(
        'build', '--locked', '--offline', '--release',
        '-p', 'sico-cli',
        '-p', 'sico-app-cli',
        '-p', 'sico-language-server',
        '-p', 'sico-ai-tools'
    ) -Description 'build Windows release tools'

    $releaseBin = Join-Path $repositoryRoot 'target/release'
    $binaryNames = @('sico.exe', 'sico-app.exe', 'sico-lsp.exe', 'sico-ai-tool.exe')
    foreach ($binaryName in $binaryNames) {
        $binaryPath = Join-Path $releaseBin $binaryName
        if (-not (Test-Path -LiteralPath $binaryPath -PathType Leaf)) {
            throw "Release binary is missing: $binaryPath"
        }
    }

    if ($RuntimePath) {
        $resolvedRuntime = (Resolve-Path -LiteralPath $RuntimePath -ErrorAction Stop).Path
    } else {
        $runtimeOutput = @(& (Join-Path $PSScriptRoot 'ensure-wasmtime.ps1') -RepositoryRoot $repositoryRoot)
        $resolvedRuntime = [string]$runtimeOutput[-1]
    }
    if (-not (Test-Path -LiteralPath $resolvedRuntime -PathType Leaf)) {
        throw "Wasmtime Runtime is missing: $resolvedRuntime"
    }
    $runtimeVersionOutput = ((& $resolvedRuntime --version) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0 -or $runtimeVersionOutput -notmatch '^wasmtime 46\.0\.1\b') {
        throw "Expected Wasmtime 46.0.1, got: $runtimeVersionOutput"
    }
    $runtimeDirectory = Split-Path -Parent $resolvedRuntime
    $runtimeLicense = Join-Path $runtimeDirectory 'LICENSE'
    if (-not (Test-Path -LiteralPath $runtimeLicense -PathType Leaf)) {
        throw "Wasmtime license is missing beside the Runtime: $runtimeLicense"
    }

    $packagingRoot = Join-Path $repositoryRoot "target/release-packaging/$tag"
    Reset-Directory -SafeParent (Join-Path $repositoryRoot 'target') -Path $packagingRoot
    $smokeRoot = Join-Path $packagingRoot 'smoke'
    New-Item -ItemType Directory -Path $smokeRoot | Out-Null
    $smokeSource = Join-Path $smokeRoot 'one-plus-two.sico'
    $smokeComponent = Join-Path $smokeRoot 'one-plus-two.component.wasm'
    $smokePackage = Join-Path $smokeRoot 'one-plus-two.sapp'
    $utf8NoBom = New-Object System.Text.UTF8Encoding($false)
    [IO.File]::WriteAllText(
        $smokeSource,
        "function main() returns Int:`n  return 1 + 2`nend function`n",
        $utf8NoBom
    )

    $sico = Join-Path $releaseBin 'sico.exe'
    $sicoApp = Join-Path $releaseBin 'sico-app.exe'
    Invoke-Checked -Executable $sico -Arguments @('build', '-o', $smokeComponent, $smokeSource) -Description 'compile release smoke program'
    Invoke-Checked -Executable $sicoApp -Arguments @(
        'pack', '--app-id', 'dev.sico.release-smoke', '--app-version', $Version,
        '-o', $smokePackage, $smokeComponent
    ) -Description 'package release smoke program'
    $smokeOutput = @(& $sicoApp run --allow-unsigned-dev --runtime $resolvedRuntime $smokePackage)
    $smokeExitCode = $LASTEXITCODE
    $smokeResult = ($smokeOutput -join "`n").Trim()
    if ($smokeExitCode -ne 0 -or $smokeResult -cne '3') {
        throw "Release smoke failed: exit=$smokeExitCode output='$smokeResult'"
    }
    Write-Host '==> release smoke result: 3'

    if (-not $OutputRoot) {
        $OutputRoot = Join-Path $repositoryRoot 'dist'
    } elseif (-not [IO.Path]::IsPathRooted($OutputRoot)) {
        $OutputRoot = Join-Path $repositoryRoot $OutputRoot
    }
    $OutputRoot = [IO.Path]::GetFullPath($OutputRoot)
    Assert-SafeChildPath -Parent $repositoryRoot -Child $OutputRoot
    New-Item -ItemType Directory -Force -Path $OutputRoot | Out-Null
    $releaseOutput = Join-Path $OutputRoot $tag
    Reset-Directory -SafeParent $OutputRoot -Path $releaseOutput

    $compilerPayload = Join-Path $packagingRoot 'compiler-payload'
    $sdkPayload = Join-Path $packagingRoot 'sdk-payload'
    New-Item -ItemType Directory -Path (Join-Path $compilerPayload 'bin') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $compilerPayload 'docs') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $sdkPayload 'bin') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $sdkPayload 'runtime') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $sdkPayload 'docs') -Force | Out-Null
    New-Item -ItemType Directory -Path (Join-Path $sdkPayload 'licenses') -Force | Out-Null

    Copy-Item -LiteralPath $sico -Destination (Join-Path $compilerPayload 'bin')
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'LICENSE') -Destination $compilerPayload
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'docs/user-guide/INSTALLATION.md') -Destination (Join-Path $compilerPayload 'docs')

    foreach ($binaryName in $binaryNames) {
        Copy-Item -LiteralPath (Join-Path $releaseBin $binaryName) -Destination (Join-Path $sdkPayload 'bin')
    }
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'sico-dev.ps1') -Destination (Join-Path $sdkPayload 'bin')
    $cmdLauncher = @(
        '@echo off',
        'powershell.exe -NoProfile -File "%~dp0sico-dev.ps1" %*',
        'exit /b %ERRORLEVEL%'
    ) -join "`r`n"
    [IO.File]::WriteAllText((Join-Path $sdkPayload 'bin/sico-dev.cmd'), "$cmdLauncher`r`n", [Text.Encoding]::ASCII)
    Copy-Item -LiteralPath $resolvedRuntime -Destination (Join-Path $sdkPayload 'runtime/wasmtime.exe')
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'LICENSE') -Destination $sdkPayload
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'install-windows.ps1') -Destination $sdkPayload
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'install-windows.cmd') -Destination $sdkPayload
    Copy-Item -LiteralPath $runtimeLicense -Destination (Join-Path $sdkPayload 'licenses/WASMTIME-LICENSE')
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'docs/user-guide/INSTALLATION.md') -Destination (Join-Path $sdkPayload 'docs')
    Copy-Item -LiteralPath (Join-Path $repositoryRoot 'docs/user-guide/BUILD-RUN.md') -Destination (Join-Path $sdkPayload 'docs')

    $compilerReadme = @(
        "Sico compiler $Version ($targetTriple)",
        '',
        'Add the bin directory to PATH, then run:',
        '  sico --version',
        '  sico check hello.sico',
        '  sico build hello.sico',
        '',
        "Commit: $commit",
        'Code signing: unsigned development release'
    ) -join "`r`n"
    [IO.File]::WriteAllText((Join-Path $compilerPayload 'README.txt'), "$compilerReadme`r`n", $utf8NoBom)

    $sdkReadme = @(
        "Sico SDK $Version ($targetTriple)",
        '',
        'Install for the current user and add the bin directory to PATH:',
        '  .\install-windows.cmd',
        '',
        'Open a new terminal, then run:',
        '  sico --version',
        '  sico-dev hello.sico',
        '',
        'The SDK contains compiler, application CLI, LSP, AI tool, development runner,',
        'and the verified Wasmtime 46.0.1 Runtime.',
        '',
        "Commit: $commit",
        'Code signing: unsigned development release'
    ) -join "`r`n"
    [IO.File]::WriteAllText((Join-Path $sdkPayload 'README.txt'), "$sdkReadme`r`n", $utf8NoBom)

    $thirdPartyNotice = @(
        'Third-party notices',
        '',
        'The Sico SDK redistributes Wasmtime 46.0.1 from the Bytecode Alliance.',
        'Its license is included at licenses/WASMTIME-LICENSE.',
        'Source: https://github.com/bytecodealliance/wasmtime',
        '',
        'Cargo dependency licenses are recorded in SBOM.spdx.json.'
    ) -join "`r`n"
    [IO.File]::WriteAllText((Join-Path $sdkPayload 'THIRD-PARTY-NOTICES.txt'), "$thirdPartyNotice`r`n", $utf8NoBom)

    $compilerArchiveName = "sico-compiler-$tag-$targetTriple.zip"
    $sdkArchiveName = "sico-sdk-$tag-$targetTriple.zip"
    $setupName = "sico-init-$tag-$targetTriple.exe"
    $compilerArchive = Join-Path $releaseOutput $compilerArchiveName
    $sdkArchive = Join-Path $releaseOutput $sdkArchiveName
    $setupPath = Join-Path $releaseOutput $setupName
    Write-Host '==> create release archives'
    Compress-Archive -Path (Join-Path $compilerPayload '*') -DestinationPath $compilerArchive -CompressionLevel Optimal
    Compress-Archive -Path (Join-Path $sdkPayload '*') -DestinationPath $sdkArchive -CompressionLevel Optimal

    $extractRoot = Join-Path $packagingRoot 'extracted-sdk'
    New-Item -ItemType Directory -Path $extractRoot | Out-Null
    Expand-Archive -LiteralPath $sdkArchive -DestinationPath $extractRoot
    $extractedSico = Join-Path $extractRoot 'bin/sico.exe'
    $extractedDev = Join-Path $extractRoot 'bin/sico-dev.ps1'
    $extractedVersion = ((& $extractedSico --version) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0 -or $extractedVersion -cne "sico $Version") {
        throw "Extracted compiler version check failed: $extractedVersion"
    }
    $extractedSmokeOutput = @(& $extractedDev $smokeSource)
    $extractedSmokeExitCode = $LASTEXITCODE
    $extractedSmokeResult = ($extractedSmokeOutput -join "`n").Trim()
    if ($extractedSmokeExitCode -ne 0 -or $extractedSmokeResult -cne '3') {
        throw "Extracted SDK smoke failed: exit=$extractedSmokeExitCode output='$extractedSmokeResult'"
    }
    Write-Host '==> extracted SDK smoke result: 3'

    & (Join-Path $PSScriptRoot 'internal/package-windows-installer.ps1') `
        -SdkArchive $sdkArchive `
        -Version $Version `
        -OutputPath $setupPath `
        -WorkDirectory (Join-Path $packagingRoot 'windows-installer')

    $packageIdMap = @{}
    $sbomPackages = @()
    $packageIndex = 0
    foreach ($package in @($metadata.packages | Sort-Object name, version, id)) {
        $packageIndex += 1
        $safeName = ([string]$package.name) -replace '[^A-Za-z0-9.-]', '-'
        $spdxId = "SPDXRef-Package-$packageIndex-$safeName"
        $packageIdMap[[string]$package.id] = $spdxId
        $downloadLocation = 'NOASSERTION'
        if ($package.source -and ([string]$package.source).StartsWith('registry+')) {
            $downloadLocation = "https://crates.io/crates/$($package.name)/$($package.version)"
        }
        $declaredLicense = if ($package.license) { [string]$package.license } else { 'NOASSERTION' }
        $sbomPackages += [ordered]@{
            name = [string]$package.name
            SPDXID = $spdxId
            versionInfo = [string]$package.version
            downloadLocation = $downloadLocation
            filesAnalyzed = $false
            licenseConcluded = 'NOASSERTION'
            licenseDeclared = $declaredLicense
            copyrightText = 'NOASSERTION'
        }
    }
    $relationships = @()
    foreach ($package in $releasePackages) {
        $relationships += [ordered]@{
            spdxElementId = 'SPDXRef-DOCUMENT'
            relationshipType = 'DESCRIBES'
            relatedSpdxElement = $packageIdMap[[string]$package.id]
        }
    }
    foreach ($node in @($metadata.resolve.nodes)) {
        $sourceId = $packageIdMap[[string]$node.id]
        foreach ($dependency in @($node.deps)) {
            $targetId = $packageIdMap[[string]$dependency.pkg]
            if ($sourceId -and $targetId) {
                $relationships += [ordered]@{
                    spdxElementId = $sourceId
                    relationshipType = 'DEPENDS_ON'
                    relatedSpdxElement = $targetId
                }
            }
        }
    }
    $sbom = [ordered]@{
        spdxVersion = 'SPDX-2.3'
        dataLicense = 'CC0-1.0'
        SPDXID = 'SPDXRef-DOCUMENT'
        name = "Sico SDK $Version $targetTriple"
        documentNamespace = "https://github.com/ImmortalWing/sico/releases/tag/$tag/sbom/$commit"
        creationInfo = [ordered]@{
            created = $created
            creators = @('Tool: tools/release-windows.ps1')
        }
        packages = $sbomPackages
        relationships = $relationships
    }
    $sbomPath = Join-Path $releaseOutput 'SBOM.spdx.json'
    [IO.File]::WriteAllText($sbomPath, (($sbom | ConvertTo-Json -Depth 100) + "`n"), $utf8NoBom)

    $compilerAsset = Get-AssetRecord -Path $compilerArchive -Role 'compiler'
    $sdkAsset = Get-AssetRecord -Path $sdkArchive -Role 'sdk'
    $setupAsset = Get-AssetRecord -Path $setupPath -Role 'windows-installer'
    $sbomAsset = Get-AssetRecord -Path $sbomPath -Role 'sbom'
    $publishable = (-not $dirty) -and (-not $SkipQualityGates)
    $manifest = [ordered]@{
        schema = 'sico.toolchain-release.v1'
        version = $Version
        tag = $tag
        channel = $channel
        commit = $commit
        target = $targetTriple
        created = $created
        publishable = $publishable
        codeSigning = 'unsigned'
        runtime = [ordered]@{
            name = 'wasmtime'
            version = '46.0.1'
            bundledIn = $sdkArchiveName
        }
        checks = [ordered]@{
            cleanWorktree = (-not $dirty)
            qualityGates = (-not $SkipQualityGates)
            sourceRuntimeSmoke = '3'
            extractedSdkSmoke = '3'
        }
        assets = @($compilerAsset, $sdkAsset, $setupAsset, $sbomAsset)
    }
    $manifestPath = Join-Path $releaseOutput 'release-manifest.json'
    [IO.File]::WriteAllText($manifestPath, (($manifest | ConvertTo-Json -Depth 20) + "`n"), $utf8NoBom)

    $releaseNotes = @(
        "# Sico $tag",
        '',
        ('- Channel: `{0}`' -f $channel),
        ('- Commit: `{0}`' -f $commit),
        ('- Target: `{0}`' -f $targetTriple),
        '- Runtime: Wasmtime 46.0.1 (SDK bundle)',
        '- Code signing: unsigned development release',
        '',
        '## Assets',
        '',
        ('- `{0}`: compiler only' -f $compilerArchiveName),
        ('- `{0}`: compiler, application tools, LSP, AI tool, development runner and Runtime' -f $sdkArchiveName),
        ('- `{0}`: double-click, offline Windows user installer' -f $setupName),
        '- `SHA256SUMS`: exact asset hashes',
        '- `SBOM.spdx.json`: SPDX 2.3 dependency inventory',
        '- `release-manifest.json`: machine-readable release identity and checks',
        '',
        '## Quick start',
        '',
        'Double-click the `sico-init` EXE. After installation, open a new terminal and run:',
        '',
        '```powershell',
        'sico --version',
        'sico-dev hello.sico',
        '```',
        '',
        'This preview is verified only on Windows x86_64 with the target listed above.'
    ) -join "`n"
    $releaseNotesPath = Join-Path $releaseOutput 'RELEASE-NOTES.md'
    [IO.File]::WriteAllText($releaseNotesPath, "$releaseNotes`n", $utf8NoBom)

    $checksumFiles = @(
        $compilerArchive,
        $sdkArchive,
        $setupPath,
        $sbomPath,
        $manifestPath,
        $releaseNotesPath
    ) | Sort-Object { Split-Path -Leaf $_ }
    $checksumLines = foreach ($file in $checksumFiles) {
        $hash = (Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $(Split-Path -Leaf $file)"
    }
    [IO.File]::WriteAllText(
        (Join-Path $releaseOutput 'SHA256SUMS'),
        (($checksumLines -join "`n") + "`n"),
        [Text.Encoding]::ASCII
    )

    if (-not $publishable) {
        $reasons = @()
        if ($dirty) { $reasons += 'worktree was dirty (-AllowDirty)' }
        if ($SkipQualityGates) { $reasons += 'quality gates were skipped (-SkipQualityGates)' }
        [IO.File]::WriteAllText(
            (Join-Path $releaseOutput 'DO-NOT-PUBLISH.txt'),
            "This build is not publishable: $($reasons -join '; ').`r`n",
            $utf8NoBom
        )
    }

    if (Test-Path -LiteralPath $packagingRoot) {
        Assert-SafeChildPath -Parent (Join-Path $repositoryRoot 'target') -Child $packagingRoot
        Remove-Item -LiteralPath $packagingRoot -Recurse -Force
    }

    Write-Host ''
    Write-Host "RELEASE_WINDOWS_OK version=$Version target=$targetTriple publishable=$($publishable.ToString().ToLowerInvariant())"
    Write-Host "Output: $releaseOutput"
    Get-ChildItem -LiteralPath $releaseOutput | Select-Object Name, Length
} finally {
    Pop-Location
}
