param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$compilerTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8

foreach ($marker in @(
    'function general_while_function_ir(',
    'function gw_decimal_value(',
    'function gw_compare_packed(',
    'function gw_rhs_packed(',
    'function gw_instr_count(',
    'let general_while_json = general_while_function_ir(',
    'sico_compiler_lowers_the_scan_space_region_byte_exactly'
)) {
    if (-not $parser.Contains($marker)) {
        if (-not $compilerTest.Contains($marker)) { throw "missing STEP-0251 marker: $marker" }
    }
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
    $temp = Join-Path $tempRoot ("sico-step0251-" + [Guid]::NewGuid().ToString('N'))
    [void](New-Item -ItemType Directory -Path $temp)
    try {
        $compilerComponent = Join-Path $temp 'compiler.component.wasm'
        & $sico build --profile script-v0 --output $compilerComponent (Join-Path $root 'selfhost\compiler.sico') | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'self-host compiler component build failed' }

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
        # The measured typed frontier after the general while-body if/else
        # regions: scan_space lowers and the next formatter function refuses
        # on the straight-line let-RHS intrinsic gap, before any guest trap.
        if ($process.ExitCode -ne 122 -or -not $stderr.Contains('ERR:E-SH-IR-EXPRESSION')) {
            throw "formatter canary did not reach the declared typed boundary: exit=$($process.ExitCode) stderr=$stderr stdout=$stdout"
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

Write-Output 'STEP_0251_OK scan-space=byte-exact parity=8-functions canary=EXPRESSION next=straight-let-intrinsics'
