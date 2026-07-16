[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)]
    [string]$SdkArchive,

    [Parameter(Mandatory = $true)]
    [string]$Version,

    [Parameter(Mandatory = $true)]
    [string]$OutputPath,

    [Parameter(Mandatory = $true)]
    [string]$WorkDirectory
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Resolve-NativeTool {
    param([Parameter(Mandatory = $true)][string]$Name)
    return (Get-Command $Name -ErrorAction Stop).Source
}

function Assert-SafeDirectory {
    param([Parameter(Mandatory = $true)][string]$Path)

    $fullPath = [IO.Path]::GetFullPath($Path).TrimEnd('\')
    $root = [IO.Path]::GetPathRoot($fullPath).TrimEnd('\')
    if ($fullPath -eq $root -or $fullPath.Length -le ($root.Length + 3)) {
        throw "Unsafe work directory: $fullPath"
    }
    return $fullPath
}

if ($Version -notmatch '^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$') {
    throw "Version is not canonical SemVer: $Version"
}

$sdkArchivePath = (Resolve-Path -LiteralPath $SdkArchive -ErrorAction Stop).Path
$sdkArchiveName = Split-Path -Leaf $sdkArchivePath
$sourceDirectory = (Split-Path -Parent $sdkArchivePath).TrimEnd('\') + '\'
$outputFile = [IO.Path]::GetFullPath($OutputPath)
$outputDirectory = Split-Path -Parent $outputFile
$workRoot = Assert-SafeDirectory $WorkDirectory

if (-not (Test-Path -LiteralPath $outputDirectory -PathType Container)) {
    throw "Installer output directory does not exist: $outputDirectory"
}
if (Test-Path -LiteralPath $outputFile) {
    throw "Refusing to overwrite installer output: $outputFile"
}
if (Test-Path -LiteralPath $workRoot) {
    throw "Installer work directory already exists: $workRoot"
}
New-Item -ItemType Directory -Path $workRoot | Out-Null

$iexpress = Resolve-NativeTool -Name 'iexpress.exe'
$powershell = Resolve-NativeTool -Name 'powershell.exe'
$tag = "v$Version"
$bootstrapPath = Join-Path $sourceDirectory 'sico-setup-bootstrap.ps1'
$bootstrapTemplate = Join-Path $PSScriptRoot 'windows-installer-bootstrap.ps1'
$sedPath = Join-Path $workRoot 'sico-setup.sed'
$validationInstall = Join-Path $workRoot 'installed-sdk'
$originalUserPath = [Environment]::GetEnvironmentVariable('Path', [EnvironmentVariableTarget]::User)
$previousNoUi = $env:SICO_SETUP_NO_UI
$previousInstallDir = $env:SICO_SETUP_INSTALL_DIR
$completed = $false

try {
    if (Test-Path -LiteralPath $bootstrapPath) {
        throw "Temporary bootstrap path already exists: $bootstrapPath"
    }

    Copy-Item -LiteralPath $bootstrapTemplate -Destination $bootstrapPath

    $sed = @"
[Version]
Class=IEXPRESS
SEDVersion=3

[Options]
PackagePurpose=InstallApp
ShowInstallProgramWindow=0
HideExtractAnimation=1
UseLongFileName=1
InsideCompressed=0
CAB_FixedSize=0
CAB_ResvCodeSigning=0
RebootMode=N
InstallPrompt=%InstallPrompt%
DisplayLicense=%DisplayLicense%
FinishMessage=%FinishMessage%
TargetName=%TargetName%
FriendlyName=%FriendlyName%
AppLaunched=%AppLaunched%
PostInstallCmd=<None>
AdminQuietInstCmd=
UserQuietInstCmd=
SourceFiles=SourceFiles

[Strings]
InstallPrompt=
DisplayLicense=
FinishMessage=
TargetName=$outputFile
FriendlyName=Sico $tag Installer
AppLaunched=powershell.exe -NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File sico-setup-bootstrap.ps1 -Archive $sdkArchiveName
FILE0=sico-setup-bootstrap.ps1
FILE1=$sdkArchiveName

[SourceFiles]
SourceFiles0=$sourceDirectory

[SourceFiles0]
%FILE0%=
%FILE1%=
"@
    [IO.File]::WriteAllText($sedPath, $sed, [Text.Encoding]::ASCII)

    Write-Host '==> create self-contained Windows installer'
    & $iexpress /N /Q $sedPath
    if ($LASTEXITCODE -ne 0) {
        throw "IExpress failed with exit code $LASTEXITCODE"
    }

    # IExpress returns before its cabinet worker. Wait until the output is no
    # longer locked so the bootstrap and SDK archive remain available.
    $deadline = [DateTimeOffset]::UtcNow.AddMinutes(3)
    $ready = $false
    while (-not $ready -and [DateTimeOffset]::UtcNow -lt $deadline) {
        if (Test-Path -LiteralPath $outputFile -PathType Leaf) {
            try {
                $stream = [IO.File]::Open(
                    $outputFile,
                    [IO.FileMode]::Open,
                    [IO.FileAccess]::Read,
                    [IO.FileShare]::None
                )
                $stream.Dispose()
                $ready = $true
                continue
            } catch [IO.IOException] {}
        }
        Start-Sleep -Milliseconds 250
    }
    if (-not $ready) {
        throw "Windows installer was not created or remained locked: $outputFile"
    }

    $env:SICO_SETUP_NO_UI = '1'
    $env:SICO_SETUP_INSTALL_DIR = $validationInstall
    Write-Host '==> validate self-contained Windows installer'
    $setupProcess = Start-Process -FilePath $outputFile -Wait -PassThru -WindowStyle Hidden
    if ($setupProcess.ExitCode -ne 0) {
        throw "Windows installer failed with exit code $($setupProcess.ExitCode)"
    }
    $installedVersion = ((& (Join-Path $validationInstall 'bin/sico.exe') --version) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0 -or $installedVersion -cne "sico $Version") {
        throw "Windows installer version check failed: $installedVersion"
    }

    Write-Host '==> validate Windows installer uninstall'
    & $powershell -NoProfile -ExecutionPolicy Bypass -File `
        (Join-Path $validationInstall 'install-windows.ps1') `
        -Action Uninstall -InstallDirectory $validationInstall
    if ($LASTEXITCODE -ne 0) {
        throw "Windows installer uninstall failed with exit code $LASTEXITCODE"
    }
    if (Test-Path -LiteralPath $validationInstall) {
        throw "Windows installer uninstall left files behind: $validationInstall"
    }

    $completed = $true
    Write-Host "WINDOWS_INSTALLER_OK version=$Version output=$outputFile"
} finally {
    $env:SICO_SETUP_NO_UI = $previousNoUi
    $env:SICO_SETUP_INSTALL_DIR = $previousInstallDir
    [Environment]::SetEnvironmentVariable('Path', $originalUserPath, [EnvironmentVariableTarget]::User)
    Remove-Item -LiteralPath $bootstrapPath -Force -ErrorAction SilentlyContinue
    if (Test-Path -LiteralPath $validationInstall) {
        Remove-Item -LiteralPath $validationInstall -Recurse -Force
    }
    if (-not $completed -and (Test-Path -LiteralPath $outputFile)) {
        Remove-Item -LiteralPath $outputFile -Force
    }
}
