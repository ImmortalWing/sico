[CmdletBinding()]
param([Parameter(Mandatory = $true)][string]$Archive)

$ErrorActionPreference = 'Stop'
$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) ('sico-setup-' + [guid]::NewGuid().ToString('N'))
$noUi = $env:SICO_SETUP_NO_UI -eq '1'

try {
    New-Item -ItemType Directory -Path $temporaryRoot | Out-Null
    Expand-Archive -LiteralPath (Join-Path $PSScriptRoot $Archive) -DestinationPath $temporaryRoot
    $arguments = @{ Action = 'Install'; Scope = 'User' }
    if (-not [string]::IsNullOrWhiteSpace($env:SICO_SETUP_INSTALL_DIR)) {
        $arguments.InstallDirectory = $env:SICO_SETUP_INSTALL_DIR
    }
    & (Join-Path $temporaryRoot 'install-windows.ps1') @arguments
    $installedRoot = if (-not [string]::IsNullOrWhiteSpace($env:SICO_SETUP_INSTALL_DIR)) {
        $env:SICO_SETUP_INSTALL_DIR
    } else {
        Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Programs/Sico'
    }
    $version = ((& (Join-Path $installedRoot 'bin/sico.exe') --version) -join "`n").Trim()
    if ($LASTEXITCODE -ne 0 -or $version -notmatch '^sico\s+\S+$') {
        throw "Compiler verification failed: $version"
    }
    if (-not $noUi) {
        Add-Type -AssemblyName System.Windows.Forms
        [void][Windows.Forms.MessageBox]::Show(
            "Sico installation completed.`n`n$version`n`nOpen a new terminal and run: sico --version",
            'Sico Installer',
            [Windows.Forms.MessageBoxButtons]::OK,
            [Windows.Forms.MessageBoxIcon]::Information
        )
    }
    exit 0
} catch {
    if (-not $noUi) {
        try {
            Add-Type -AssemblyName System.Windows.Forms
            [void][Windows.Forms.MessageBox]::Show(
                "Sico installation failed.`n`n$($_.Exception.Message)",
                'Sico Installer',
                [Windows.Forms.MessageBoxButtons]::OK,
                [Windows.Forms.MessageBoxIcon]::Error
            )
        } catch {}
    }
    exit 1
} finally {
    if (Test-Path -LiteralPath $temporaryRoot) {
        Remove-Item -LiteralPath $temporaryRoot -Recurse -Force
    }
}
