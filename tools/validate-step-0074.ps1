$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$root = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
$cargo = (Resolve-Path (Join-Path $HOME '.cargo/bin/cargo.exe')).Path
$targetRoot = [IO.Path]::GetFullPath((Join-Path $root 'target'))
$smokeRoot = [IO.Path]::GetFullPath((Join-Path $targetRoot 'step-0074-smoke'))
$bundleRoot = [IO.Path]::GetFullPath((Join-Path $targetRoot 'step-0074-bundle-validation'))
$targetPrefix = $targetRoot.TrimEnd('\') + '\'
foreach ($path in @($smokeRoot, $bundleRoot)) {
    if (-not $path.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Unsafe STEP-0074 path: $path"
    }
}
foreach ($path in @($smokeRoot, $bundleRoot)) {
    if (Test-Path -LiteralPath $path) {
        Remove-Item -LiteralPath $path -Recurse -Force
    }
}
New-Item -ItemType Directory -Path (Join-Path $smokeRoot 'records/releases') -Force | Out-Null
$utf8 = [Text.UTF8Encoding]::new($false)
[IO.File]::WriteAllText((Join-Path $smokeRoot 'records/releases/probe.json'), "{`"probe`":true}`n", $utf8)

Push-Location $root
$process = $null
try {
    & $cargo build --locked --offline -p sico-cli -p sico-app-cli -p sico-registry-server
    if ($LASTEXITCODE -ne 0) { throw 'STEP-0074 debug build failed' }

    $listener = [Net.Sockets.TcpListener]::new([Net.IPAddress]::Loopback, 0)
    $listener.Start()
    $port = ([Net.IPEndPoint]$listener.LocalEndpoint).Port
    $listener.Stop()
    $origin = Join-Path $root 'target/debug/sico-registry.exe'
    $process = Start-Process -FilePath $origin -ArgumentList @(
        'serve', '--root', $smokeRoot, '--listen', "127.0.0.1:$port"
    ) -PassThru -WindowStyle Hidden

    $ready = $null
    for ($attempt = 0; $attempt -lt 40; $attempt += 1) {
        try {
            $ready = Invoke-RestMethod -Uri "http://127.0.0.1:$port/readyz" -TimeoutSec 1
            break
        } catch {
            Start-Sleep -Milliseconds 100
        }
    }
    if (-not $ready -or $ready.status -cne 'ready') { throw 'origin readiness probe failed' }
    $probe = Invoke-RestMethod -Uri "http://127.0.0.1:$port/v1/records/releases/probe.json" -TimeoutSec 2
    if (-not $probe.probe) { throw 'origin immutable object probe failed' }
    try {
        Invoke-WebRequest -Method Post -Uri "http://127.0.0.1:$port/healthz" -TimeoutSec 2 | Out-Null
        throw 'origin accepted a mutation method'
    } catch {
        if ($_.Exception.Response.StatusCode.value__ -ne 405) { throw }
    }

    $runtime = & (Join-Path $root 'tools/ensure-wasmtime.ps1')
    $output = @(& (Join-Path $root 'target/debug/sico-app.exe') dev `
        --compiler (Join-Path $root 'target/debug/sico.exe') `
        --runtime $runtime `
        --app-id dev.sico.step0074 `
        --app-version 0.0.2 `
        (Join-Path $root 'tests/end-to-end/answer.sico'))
    if ($LASTEXITCODE -ne 0 -or (($output -join "`n").Trim()) -cne '42') {
        throw "one-command dev smoke failed: $($output -join ' ')"
    }

    & (Join-Path $root 'tools/package-registry-origin.ps1') -OutputRoot $bundleRoot
    $bundle = Join-Path $bundleRoot 'sico-registry-origin-v0.0.2-dev-x86_64-pc-windows-gnu.zip'
    $firstBundleHash = (Get-FileHash -LiteralPath $bundle -Algorithm SHA256).Hash
    & (Join-Path $root 'tools/package-registry-origin.ps1') -OutputRoot $bundleRoot
    $secondBundleHash = (Get-FileHash -LiteralPath $bundle -Algorithm SHA256).Hash
    if ($firstBundleHash -cne $secondBundleHash) { throw 'origin operator bundle is not deterministic' }
    $bundleManifest = Get-Content -LiteralPath (Join-Path $bundleRoot 'registry-origin-manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($bundleManifest.publicProductionEvidence -ne $false -or $bundleManifest.sha256 -cne $secondBundleHash.ToLowerInvariant()) {
        throw 'origin operator bundle manifest is inconsistent'
    }

    $status = Get-Content -LiteralPath (Join-Path $root 'docs/STATUS.md') -Raw -Encoding UTF8
    $roadmap = Get-Content -LiteralPath (Join-Path $root 'docs/ROADMAP.md') -Raw -Encoding UTF8
    $step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0074-production-deployment-origin-and-ux.md') -Raw -Encoding UTF8
    if ($status -notmatch 'STEP-0074' -or $roadmap -notmatch 'M7 production deployment continuation') {
        throw 'status/roadmap is not synchronized to STEP-0074'
    }
    if ($step -notmatch 'publicly deployed production service') {
        throw 'STEP-0074 evidence boundary is missing'
    }
    Write-Host 'STEP_0074_OK origin=loopback dev_result=42 bundle=deterministic public_production=false'
} finally {
    if ($process -and -not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit()
    }
    Pop-Location
    foreach ($path in @($smokeRoot, $bundleRoot)) {
        if (Test-Path -LiteralPath $path) {
            Remove-Item -LiteralPath $path -Recurse -Force
        }
    }
}
