param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$compilerTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8

foreach ($marker in @(
    'function user_call_expression_json(',
    'environment_call_arguments(words, args_start, close',
    'if is_word(return_kind, "string")',
    '.find("function scan_space")'
)) {
    if (-not ($parser + $compilerTest).Contains($marker)) { throw "missing STEP-0248 marker: $marker" }
}

Push-Location $root
try {
    $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
    & (Join-Path $root 'tools\update-m22-corpus.ps1') -Verify
    if ($LASTEXITCODE -ne 0) { throw 'M22 corpus verification failed' }
    & $CargoPath build --locked --offline -p sico-cli
    if ($LASTEXITCODE -ne 0) { throw 'sico CLI build failed' }
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'STEP-0248 self-host compiler regressions failed' }

    $sico = Join-Path $root 'target\debug\sico.exe'
    $runner = Join-Path $root 'runner\sico-runner\target\debug\sico-runner.exe'
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $temp = Join-Path $tempRoot ("sico-step0248-" + [Guid]::NewGuid().ToString('N'))
    [void](New-Item -ItemType Directory -Path $temp)
    try {
        $component = Join-Path $temp 'compiler.component.wasm'
        & $sico build --profile script-v0 --output $component (Join-Path $root 'selfhost\compiler.sico') | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'self-host compiler component build failed' }

        $start = [Diagnostics.ProcessStartInfo]::new()
        $start.FileName = $runner
        # Windows PowerShell 5.1 has no ProcessStartInfo.ArgumentList.
        $start.Arguments = ('--fuel 5000000000 "' + $component + '"')
        $start.RedirectStandardInput = $true
        $start.RedirectStandardOutput = $true
        $start.RedirectStandardError = $true
        $start.UseShellExecute = $false
        $process = [Diagnostics.Process]::Start($start)
        $process.StandardInput.Write([IO.File]::ReadAllText((Join-Path $root 'selfhost\formatter.sico')))
        $process.StandardInput.Close()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        # Frontier note (STEP-0249): the sbi extraction advanced the typed
        # canary boundary from E-SH-IR-EXPRESSION to E-SH-IR-CALL-TYPE; the
        # assertion tracks the current measured frontier, still fail-closed.
        if ($process.ExitCode -ne 122 -or -not $stderr.Contains('ERR:E-SH-IR-CALL-TYPE')) {
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

Write-Output 'STEP_0248_OK compiler=7/7 formatter-prefix=7 call-guards=typed string-returns=canonical trap=absent next=while-scan'
