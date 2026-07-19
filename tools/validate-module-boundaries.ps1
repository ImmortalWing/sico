$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$contractPath = Join-Path $root 'tests/architecture/module-boundaries.json'
$contract = Get-Content -LiteralPath $contractPath -Raw -Encoding UTF8 | ConvertFrom-Json
if ($contract.schema -ne 'sico.module-boundaries.v1' -or $contract.model -ne 'openjdk-style-modular-monorepo') {
    throw 'unexpected module-boundary contract identity'
}

$cargo = Join-Path $HOME '.cargo/bin/cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$metadata = & $cargo metadata --format-version 1 --locked --offline | ConvertFrom-Json
if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata failed'
}

$packageToModule = @{}
foreach ($moduleProperty in $contract.modules.PSObject.Properties) {
    foreach ($package in $moduleProperty.Value) {
        if ($packageToModule.ContainsKey($package)) {
            throw "package '$package' is assigned to more than one module"
        }
        $packageToModule[$package] = $moduleProperty.Name
    }
}

$workspacePackages = @($metadata.packages | Where-Object { $metadata.workspace_members -contains $_.id })
foreach ($package in $workspacePackages) {
    if (-not $packageToModule.ContainsKey($package.name)) {
        throw "workspace package '$($package.name)' has no module assignment"
    }
}
foreach ($package in $packageToModule.Keys) {
    if (-not ($workspacePackages.name -contains $package)) {
        throw "module contract references missing workspace package '$package'"
    }
}

$dependencyExceptions = @{}
foreach ($exception in @($contract.allowed_normal_dependency_exceptions)) {
    $source = [string]$exception.source
    $target = [string]$exception.target
    $reason = [string]$exception.reason
    $key = "$source->$target"
    if (-not $packageToModule.ContainsKey($source) -or -not $packageToModule.ContainsKey($target) -or
        [string]::IsNullOrWhiteSpace($reason) -or $dependencyExceptions.ContainsKey($key)) {
        throw "invalid normal dependency exception: $key"
    }
    if (@($contract.allowed_normal_dependencies.$($packageToModule[$source])) -contains $packageToModule[$target]) {
        throw "redundant normal dependency exception: $key"
    }
    $dependencyExceptions[$key] = $reason
}
$usedDependencyExceptions = @{}

foreach ($package in $workspacePackages) {
    $sourceModule = $packageToModule[$package.name]
    $allowed = @($contract.allowed_normal_dependencies.$sourceModule)
    foreach ($dependency in @($package.dependencies | Where-Object { $null -eq $_.kind })) {
        if (-not $packageToModule.ContainsKey($dependency.name)) {
            continue
        }
        $targetModule = $packageToModule[$dependency.name]
        if ($allowed -notcontains $targetModule) {
            $key = "$($package.name)->$($dependency.name)"
            if (-not $dependencyExceptions.ContainsKey($key)) {
                throw "forbidden normal dependency: $($package.name) [$sourceModule] -> $($dependency.name) [$targetModule]"
            }
            $usedDependencyExceptions[$key] = $true
        }
    }
}
foreach ($key in $dependencyExceptions.Keys) {
    if (-not $usedDependencyExceptions.ContainsKey($key)) {
        throw "stale normal dependency exception: $key"
    }
}

$languageCli = $workspacePackages | Where-Object name -eq 'sico-cli'
$languageCliDependencies = @($languageCli.dependencies | Where-Object { $null -eq $_.kind } | ForEach-Object name)
foreach ($forbidden in @('sico-runtime', 'sico-host-core', 'sico-desktop-host', 'sico-mobile-host-core')) {
    if ($languageCliDependencies -contains $forbidden) {
        throw "language CLI must not depend on $forbidden"
    }
}

$appCli = $workspacePackages | Where-Object name -eq 'sico-app-cli'
$appCliDependencies = @($appCli.dependencies | Where-Object { $null -eq $_.kind } | ForEach-Object name)
foreach ($required in @('sico-package', 'sico-runtime')) {
    if ($appCliDependencies -notcontains $required) {
        throw "application CLI must depend on $required"
    }
}

Write-Output "MODULE_BOUNDARIES_OK model=openjdk-style packages=$($workspacePackages.Count) exceptions=$($dependencyExceptions.Count) language_cli=sico app_cli=sico-app host_cli=sico-desktop-host"
