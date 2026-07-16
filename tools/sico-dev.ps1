[CmdletBinding()]
param(
    [Parameter(Mandatory = $true, Position = 0)]
    [ValidateNotNullOrEmpty()]
    [string]$Source,

    [string]$AppId = 'dev.sico.local',
    [string]$AppVersion = '0.0.2',
    [string]$Runtime,
    [string[]]$Grant = @(),
    [string]$StorageRoot,
    [switch]$KeepArtifacts,
    [switch]$Pause
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$scriptDirectory = (Resolve-Path $PSScriptRoot).Path
$repositoryRoot = (Resolve-Path (Split-Path -Parent $scriptDirectory)).Path
$sourcePath = (Resolve-Path -LiteralPath $Source -ErrorAction Stop).Path
if (-not (Test-Path -LiteralPath $sourcePath -PathType Leaf)) {
    throw "Sico source is not a file: $sourcePath"
}

$compiler = Join-Path $scriptDirectory 'sico.exe'
$appCli = Join-Path $scriptDirectory 'sico-app.exe'
if (-not (Test-Path -LiteralPath $compiler -PathType Leaf) -or
    -not (Test-Path -LiteralPath $appCli -PathType Leaf)) {
    $compiler = Join-Path $repositoryRoot 'target/release/sico.exe'
    $appCli = Join-Path $repositoryRoot 'target/release/sico-app.exe'
}
foreach ($tool in @($compiler, $appCli)) {
    if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) {
        throw "Required tool is missing: $tool`nBuild it with: cargo build --locked --release -p sico-cli -p sico-app-cli"
    }
}

if ($Runtime) {
    $runtimePath = (Resolve-Path -LiteralPath $Runtime -ErrorAction Stop).Path
} else {
    $runtimePath = Join-Path $repositoryRoot 'runtime/wasmtime.exe'
    if (-not (Test-Path -LiteralPath $runtimePath -PathType Leaf)) {
        $ensureRuntime = Join-Path $scriptDirectory 'ensure-wasmtime.ps1'
        if (-not (Test-Path -LiteralPath $ensureRuntime -PathType Leaf)) {
            throw 'Bundled Runtime is missing. Pass -Runtime <WASMTIME.exe> explicitly.'
        }
        $runtimePath = & $ensureRuntime -RepositoryRoot $repositoryRoot
    }
}
if (-not (Test-Path -LiteralPath $runtimePath -PathType Leaf)) {
    throw "Runtime is not a file: $runtimePath"
}

$temporaryRoot = Join-Path ([IO.Path]::GetTempPath()) 'sico-dev'
New-Item -ItemType Directory -Force -Path $temporaryRoot | Out-Null
$workDirectory = Join-Path $temporaryRoot ([guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $workDirectory | Out-Null

$baseName = [IO.Path]::GetFileNameWithoutExtension($sourcePath)
$component = Join-Path $workDirectory "$baseName.component.wasm"
$package = Join-Path $workDirectory "$baseName.sapp"

try {
    Write-Verbose "Compiling $sourcePath"
    & $compiler build -o $component $sourcePath | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "sico build failed with exit code $LASTEXITCODE"
    }

    Write-Verbose "Packing $package"
    & $appCli pack --app-id $AppId --app-version $AppVersion -o $package $component | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "sico-app pack failed with exit code $LASTEXITCODE"
    }

    $runArguments = @(
        'run'
        '--allow-unsigned-dev'
        '--runtime'
        $runtimePath
    )
    foreach ($capability in $Grant) {
        $runArguments += @('--grant', $capability)
    }
    if ($StorageRoot) {
        $runArguments += @('--storage-root', $StorageRoot)
    }
    $runArguments += $package

    Write-Verbose "Running $package"
    & $appCli @runArguments
    $runExitCode = $LASTEXITCODE
    if ($runExitCode -ne 0) {
        throw "sico-app run failed with exit code $runExitCode"
    }

    if ($Pause) {
        [void](Read-Host '运行结束，按 Enter 返回')
    }
} finally {
    if ($KeepArtifacts) {
        Write-Host "Artifacts: $workDirectory"
    } else {
        $resolvedTemporaryRoot = [IO.Path]::GetFullPath($temporaryRoot).TrimEnd('\') + '\'
        $resolvedWorkDirectory = [IO.Path]::GetFullPath($workDirectory)
        if (-not $resolvedWorkDirectory.StartsWith($resolvedTemporaryRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw "Refusing to remove unexpected work directory: $resolvedWorkDirectory"
        }
        Remove-Item -LiteralPath $resolvedWorkDirectory -Recurse -Force
    }
}
