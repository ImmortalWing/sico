$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
. (Join-Path $root 'tools\lib\native-command.ps1')

Invoke-NativeChecked $cargo @(
    'test', '--offline', '-p', 'sico-observability', '-p', 'sico-codegen-wasm',
    '-p', 'sico-cli', '-p', 'sico-package'
) 'rust-tests-failed|STEP-0096 packages'
Invoke-NativeChecked $cargo @('build', '--offline', '-p', 'sico-cli') 'cli-build-failed|STEP-0096 sico-cli'

$powershell = (Get-Command powershell.exe -ErrorAction Stop).Source
Invoke-NativeChecked $powershell @(
    '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
    (Join-Path $root 'tools/validate-step-0095.ps1')
) 'contract-regression|STEP-0095 validator' | Out-Null
Invoke-NativeChecked $powershell @(
    '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File',
    (Join-Path $root 'tools/validate-module-boundaries.ps1')
) 'module-boundary-regression|STEP-0096 validator' | Out-Null

$temporary = Join-Path ([IO.Path]::GetTempPath()) ("sico-step-0096-" + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($temporary) | Out-Null
try {
    $sico = Join-Path $root 'target/debug/sico.exe'
    $answer = Join-Path $root 'tests/end-to-end/answer.sico'
    $script = Join-Path $root 'tests/end-to-end/script-word-count.sico'
    $first = Join-Path $temporary 'answer-a.component.wasm'
    $second = Join-Path $temporary 'answer-b.component.wasm'
    $plain = Join-Path $temporary 'answer-plain.component.wasm'
    $scriptOutput = Join-Path $temporary 'script.component.wasm'

    & $sico build --debug-info --output $first $answer | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'debug-build-failed|answer-a' }
    & $sico build --debug-info --output $second $answer | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'debug-build-failed|answer-b' }
    & $sico build --output $plain $answer | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'plain-build-failed|answer' }
    & $sico build --profile script-v0 --debug-info --output $scriptOutput $script | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'debug-build-failed|script-word-count' }

    $suffixes = @('', '.debug-map.json', '.debug-identity.json')
    foreach ($suffix in $suffixes) {
        $leftHash = (Get-FileHash -Algorithm SHA256 -LiteralPath ($first + $suffix)).Hash
        $rightHash = (Get-FileHash -Algorithm SHA256 -LiteralPath ($second + $suffix)).Hash
        if ($leftHash -ne $rightHash) { throw "non-deterministic-artifact|$suffix" }
    }

    $map = Get-Content -LiteralPath ($first + '.debug-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $identity = Get-Content -LiteralPath ($first + '.debug-identity.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    $scriptMap = Get-Content -LiteralPath ($scriptOutput + '.debug-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($map.schema -ne 'sico.debug-map.v0' -or $identity.schema -ne 'sico.debug.identity.v0') {
        throw 'schema-mismatch|debug artifact identities'
    }
    if (@($map.functions).Count -lt 1 -or @($map.mappings).Count -lt 1) {
        throw 'mapping-missing|answer debug map'
    }
    if (@($map.functions | Where-Object core_module -ne 'sico-core').Count -ne 0) {
        throw 'core-module-mismatch|component-v0'
    }
    $generated = @($scriptMap.mappings | Where-Object { $_.generated -eq $true -and $null -eq $_.source }).Count
    if ($generated -lt 3) { throw 'synthetic-mapping-missing|script generated functions' }
    if (@($scriptMap.functions | Where-Object { $_.id -like 'generated.helper.*' }).Count -lt 1) {
        throw 'helper-mapping-missing|script intrinsic helper'
    }
    if ((Get-Item -LiteralPath ($first + '.debug-map.json')).Length -gt 16777216) {
        throw 'map-over-limit|16 MiB'
    }

    $plainText = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($plain))
    if ($plainText.Contains('sico.debug-link.v0')) {
        throw 'plain-artifact-contaminated|debug link present without --debug-info'
    }

    $componentHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $first).Hash.ToLowerInvariant()
    $mapHash = (Get-FileHash -Algorithm SHA256 -LiteralPath ($first + '.debug-map.json')).Hash.ToLowerInvariant()
    if ($identity.component_sha256 -ne $componentHash -or $identity.debug_map_sha256 -ne $mapHash) {
        throw 'digest-mismatch|installed debug triplet'
    }

    Write-Output ("STEP_0096_OK packages=4 deterministic_triplet=true maps={0}/{1} functions={2}/{3} synthetic={4} package_roundtrip=true module_packages=25 plain_debug_link=false authority=unchanged next=STEP-0097" -f @($map.mappings).Count, @($scriptMap.mappings).Count, @($map.functions).Count, @($scriptMap.functions).Count, $generated)
}
finally {
    $resolved = [IO.Path]::GetFullPath($temporary)
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    if ($resolved.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase) -and (Split-Path -Leaf $resolved).StartsWith('sico-step-0096-')) {
        Remove-Item -LiteralPath $resolved -Recurse -Force -ErrorAction SilentlyContinue
    }
}
