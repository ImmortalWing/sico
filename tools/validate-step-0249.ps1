param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8

foreach ($marker in @(
    'function while_bytes_at_rhs_packed(',
    'let sbi_pack = while_bytes_at_rhs_packed(',
    'function control_while_function_ir('
)) {
    if (-not $parser.Contains($marker)) { throw "missing STEP-0249 marker: $marker" }
}

Push-Location $root
try {
    $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
    & $CargoPath build --locked --offline -p sico-cli
    if ($LASTEXITCODE -ne 0) { throw 'sico CLI build failed' }
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_local_bounds -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host link unit local bounds regression failed' }

    $sico = Join-Path $root 'target\debug\sico.exe'
    $runner = Join-Path $root 'runner\sico-runner\target\debug\sico-runner.exe'
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $temp = Join-Path $tempRoot ("sico-step0249-" + [Guid]::NewGuid().ToString('N'))
    [void](New-Item -ItemType Directory -Path $temp)
    try {
        $driverComponent = Join-Path $temp 'parser_driver.component.wasm'
        $compilerComponent = Join-Path $temp 'compiler.component.wasm'
        foreach ($build in @(
            @($driverComponent, (Join-Path $root 'selfhost\parser_driver.sico')),
            @($compilerComponent, (Join-Path $root 'selfhost\compiler.sico'))
        )) {
            & $sico build --profile script-v0 --output $build[0] $build[1] | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "self-host component build failed: $($build[1])" }
        }

        $start = [Diagnostics.ProcessStartInfo]::new()
        $start.FileName = $runner
        # Windows PowerShell 5.1 has no ProcessStartInfo.ArgumentList.
        $start.Arguments = ('--fuel 5000000000 "' + $compilerComponent + '"')
        $start.RedirectStandardInput = $true
        $start.RedirectStandardOutput = $true
        $start.RedirectStandardError = $true
        $start.UseShellExecute = $false
        $process = [Diagnostics.Process]::Start($start)
        $stdinBytes = [Text.UTF8Encoding]::new($false).GetBytes([IO.File]::ReadAllText((Join-Path $root 'selfhost\formatter.sico')))
        $process.StandardInput.BaseStream.Write($stdinBytes, 0, $stdinBytes.Length)
        $process.StandardInput.BaseStream.Flush()
        $process.StandardInput.BaseStream.Close()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        # Frontier policy: pinned to the typed fail-closed class here; the
        # newest step validator pins the exact current error code.
        if ($process.ExitCode -ne 122 -or -not $stderr.Contains('ERR:E-SH-IR-')) {
            throw "formatter canary did not reach a typed boundary: exit=$($process.ExitCode) stderr=$stderr stdout=$stdout"
        }
        if ($stderr.Contains('"class":"trap"')) { throw 'formatter canary regressed to a guest trap' }
    }
    finally {
        $resolvedTemp = [IO.Path]::GetFullPath($temp)
        if (-not $resolvedTemp.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
            throw "refusing to remove non-temporary path: $resolvedTemp"
        }
        Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
    }

    git diff --check
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
}
finally {
    Pop-Location
}

Write-Output 'STEP_0249_OK local-limit=repaired canary=typed parser-driver=builds bounds=headroom-16 next=while-if-operands'
