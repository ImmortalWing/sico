[CmdletBinding()]
param(
    [ValidateSet('Install', 'Uninstall')]
    [string]$Action = 'Install',

    [ValidateSet('User', 'Machine')]
    [string]$Scope = 'User',

    [string]$InstallDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Test-IsAdministrator {
    $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
    $principal = [Security.Principal.WindowsPrincipal]::new($identity)
    return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Get-DefaultInstallDirectory {
    param([string]$TargetScope)

    if ($TargetScope -eq 'Machine') {
        return (Join-Path $env:ProgramFiles 'Sico')
    }
    return (Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Programs/Sico')
}

function Assert-SafeInstallDirectory {
    param([string]$Path)

    $fullPath = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    $root = [IO.Path]::GetPathRoot($fullPath).TrimEnd('\')
    if ($fullPath -eq $root -or $fullPath.Length -le ($root.Length + 3)) {
        throw "Unsafe install directory: $fullPath"
    }
    return $fullPath
}

function Normalize-PathEntry {
    param([string]$PathEntry)

    $trimmed = $PathEntry.Trim().Trim('"').TrimEnd('\')
    if ([string]::IsNullOrWhiteSpace($trimmed)) {
        return ''
    }
    try {
        return [IO.Path]::GetFullPath([Environment]::ExpandEnvironmentVariables($trimmed)).TrimEnd('\')
    } catch {
        return $trimmed
    }
}

function Update-PersistentPath {
    param(
        [string]$BinDirectory,
        [string]$TargetScope,
        [bool]$Add
    )

    $environmentTarget = if ($TargetScope -eq 'Machine') {
        [EnvironmentVariableTarget]::Machine
    } else {
        [EnvironmentVariableTarget]::User
    }
    $existing = [Environment]::GetEnvironmentVariable('Path', $environmentTarget)
    $entries = @($existing -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $normalizedBin = Normalize-PathEntry $BinDirectory
    $filtered = @($entries | Where-Object {
        (Normalize-PathEntry $_) -ine $normalizedBin
    })
    if ($Add) {
        $updated = (@($BinDirectory) + $filtered) -join ';'
    } else {
        $updated = $filtered -join ';'
    }
    [Environment]::SetEnvironmentVariable('Path', $updated, $environmentTarget)

    $processEntries = @($env:Path -split ';' | Where-Object { -not [string]::IsNullOrWhiteSpace($_) })
    $processFiltered = @($processEntries | Where-Object {
        (Normalize-PathEntry $_) -ine $normalizedBin
    })
    $env:Path = if ($Add) {
        (@($BinDirectory) + $processFiltered) -join ';'
    } else {
        $processFiltered -join ';'
    }
}

function Publish-EnvironmentChange {
    try {
        if (-not ('Sico.NativeMethods' -as [type])) {
            Add-Type -Namespace Sico -Name NativeMethods -MemberDefinition @'
[DllImport("user32.dll", SetLastError = true, CharSet = CharSet.Auto)]
public static extern IntPtr SendMessageTimeout(
    IntPtr hWnd, uint Msg, UIntPtr wParam, string lParam,
    uint fuFlags, uint uTimeout, out UIntPtr lpdwResult);
'@
        }
        $result = [UIntPtr]::Zero
        [void][Sico.NativeMethods]::SendMessageTimeout(
            [IntPtr]0xffff,
            0x001A,
            [UIntPtr]::Zero,
            'Environment',
            0x0002,
            5000,
            [ref]$result
        )
    } catch {
        Write-Verbose "Could not broadcast environment change: $($_.Exception.Message)"
    }
}

if ($Scope -eq 'Machine' -and -not (Test-IsAdministrator)) {
    throw 'Machine installation requires an Administrator PowerShell terminal. Use -Scope User or reopen PowerShell as Administrator.'
}

if ([string]::IsNullOrWhiteSpace($InstallDirectory)) {
    $InstallDirectory = Get-DefaultInstallDirectory -TargetScope $Scope
}
$installRoot = Assert-SafeInstallDirectory $InstallDirectory
$binDirectory = Join-Path $installRoot 'bin'
$markerPath = Join-Path $installRoot '.sico-install.json'

if ($Action -eq 'Uninstall') {
    if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) {
        throw "Refusing to uninstall an unmarked directory: $installRoot"
    }
    Update-PersistentPath -BinDirectory $binDirectory -TargetScope $Scope -Add $false
    Remove-Item -LiteralPath $installRoot -Recurse -Force
    Publish-EnvironmentChange
    Write-Host "SICO_UNINSTALL_OK scope=$Scope"
    Write-Host 'Open a new terminal to observe the updated PATH.'
    exit 0
}

$sourceRoot = [IO.Path]::GetFullPath($PSScriptRoot).TrimEnd('\')
$requiredFiles = @(
    'bin/sico.exe',
    'bin/sico-app.exe',
    'bin/sico-dev.ps1',
    'runtime/wasmtime.exe',
    'LICENSE'
)
foreach ($relativePath in $requiredFiles) {
    $requiredPath = Join-Path $sourceRoot $relativePath
    if (-not (Test-Path -LiteralPath $requiredPath -PathType Leaf)) {
        throw "This installer must be run from an extracted Sico SDK. Missing: $requiredPath"
    }
}
if ($sourceRoot -ieq $installRoot) {
    throw 'The SDK is already located at the install destination.'
}

$installParent = Split-Path -Parent $installRoot
New-Item -ItemType Directory -Path $installParent -Force | Out-Null
$stagingRoot = Join-Path $installParent ('.sico-staging-' + [guid]::NewGuid().ToString('N'))
$backupRoot = Join-Path $installParent ('.sico-backup-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $stagingRoot | Out-Null

try {
    Get-ChildItem -LiteralPath $sourceRoot -Force | ForEach-Object {
        Copy-Item -LiteralPath $_.FullName -Destination $stagingRoot -Recurse -Force
    }
    $versionOutput = ((& (Join-Path $stagingRoot 'bin/sico.exe') --version) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0 -or $versionOutput -notmatch '^sico\s+\S+$') {
        throw "Installed compiler validation failed: $versionOutput"
    }
    $runtimeOutput = ((& (Join-Path $stagingRoot 'runtime/wasmtime.exe') --version) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0 -or $runtimeOutput -notmatch '^wasmtime\s+46\.0\.1\b') {
        throw "Installed Runtime validation failed: $runtimeOutput"
    }

    $marker = [ordered]@{
        schema = 'sico.windows-install.v1'
        version = ($versionOutput -replace '^sico\s+', '')
        scope = $Scope
        installedAt = [DateTimeOffset]::UtcNow.ToString('yyyy-MM-ddTHH:mm:ssZ')
    }
    $utf8NoBom = [Text.UTF8Encoding]::new($false)
    [IO.File]::WriteAllText(
        (Join-Path $stagingRoot '.sico-install.json'),
        (($marker | ConvertTo-Json) + "`n"),
        $utf8NoBom
    )

    if (Test-Path -LiteralPath $installRoot) {
        if (-not (Test-Path -LiteralPath $markerPath -PathType Leaf)) {
            throw "Install destination exists but is not managed by Sico: $installRoot"
        }
        Move-Item -LiteralPath $installRoot -Destination $backupRoot
    }
    Move-Item -LiteralPath $stagingRoot -Destination $installRoot
    Update-PersistentPath -BinDirectory $binDirectory -TargetScope $Scope -Add $true
    Publish-EnvironmentChange

    if (Test-Path -LiteralPath $backupRoot) {
        Remove-Item -LiteralPath $backupRoot -Recurse -Force
    }
    Write-Host "SICO_INSTALL_OK version=$($marker.version) scope=$Scope"
    Write-Host "Installed: $installRoot"
    Write-Host "PATH added: $binDirectory"
    Write-Host 'Open a new terminal, then run: sico --version'
} catch {
    if ((Test-Path -LiteralPath $backupRoot) -and -not (Test-Path -LiteralPath $installRoot)) {
        Move-Item -LiteralPath $backupRoot -Destination $installRoot
    }
    throw
} finally {
    if (Test-Path -LiteralPath $stagingRoot) {
        Remove-Item -LiteralPath $stagingRoot -Recurse -Force
    }
}
