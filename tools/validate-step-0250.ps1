param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8

foreach ($marker in @(
    'if is_word(type_name, "Bytes"):',
    'return "bytes"',
    'function while_bytes_at_rhs_packed('
)) {
    if (-not $parser.Contains($marker)) { throw "missing STEP-0250 marker: $marker" }
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
    $temp = Join-Path $tempRoot ("sico-step0250-" + [Guid]::NewGuid().ToString('N'))
    [void](New-Item -ItemType Directory -Path $temp)
    try {
        $driverComponent = Join-Path $temp 'parser_driver.component.wasm'
        & $sico build --profile script-v0 --output $driverComponent (Join-Path $root 'selfhost\parser_driver.sico') | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'parser driver component build failed' }

        $start = [Diagnostics.ProcessStartInfo]::new()
        $start.FileName = $runner
        # Windows PowerShell 5.1 has no ProcessStartInfo.ArgumentList.
        $start.Arguments = ('--fuel 5000000000 "' + $driverComponent + '" -- --emit-ir')
        $start.RedirectStandardInput = $true
        $start.RedirectStandardOutput = $true
        $start.RedirectStandardError = $true
        $start.UseShellExecute = $false
        $process = [Diagnostics.Process]::Start($start)
        # A Bytes-parameter while-function with a fixed-width-literal set body:
        # refused as E-SH-IR-PARAMETER-TYPE before STEP-0250, lowers now.
        $bytesWhileSource = @'
function probe_bytes_while(src: Bytes, start: U64, size: U64) returns U64:
  let cursor = start
  while U64.less_than(cursor, size):
    set cursor = U64.literal(1)
  end while
  return cursor
end function
'@
        $stdinBytes = [Text.UTF8Encoding]::new($false).GetBytes($bytesWhileSource)
        $process.StandardInput.BaseStream.Write($stdinBytes, 0, $stdinBytes.Length)
        $process.StandardInput.BaseStream.Flush()
        $process.StandardInput.BaseStream.Close()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        if ($process.ExitCode -ne 0 -or -not $stdout.Contains('"name":"probe_bytes_while"')) {
            throw "Bytes-parameter while function must lower: exit=$($process.ExitCode) stderr=$stderr stdout=$stdout"
        }

        $canary = [Diagnostics.ProcessStartInfo]::new()
        $canary.FileName = $runner
        $canary.Arguments = ('--fuel 5000000000 "' + $driverComponent + '" -- --emit-ir')
        $canary.RedirectStandardInput = $true
        $canary.RedirectStandardOutput = $true
        $canary.RedirectStandardError = $true
        $canary.UseShellExecute = $false
        $canaryProcess = [Diagnostics.Process]::Start($canary)
        $formatterBytes = [Text.UTF8Encoding]::new($false).GetBytes([IO.File]::ReadAllText((Join-Path $root 'selfhost\formatter.sico')))
        $canaryProcess.StandardInput.BaseStream.Write($formatterBytes, 0, $formatterBytes.Length)
        $canaryProcess.StandardInput.BaseStream.Flush()
        $canaryProcess.StandardInput.BaseStream.Close()
        $canaryOut = $canaryProcess.StandardOutput.ReadToEnd()
        $canaryErr = $canaryProcess.StandardError.ReadToEnd()
        $canaryProcess.WaitForExit()
        # The exact current frontier: the while-body if/else statement-region
        # machinery refuses scan_space before any guest trap.
        if ($canaryProcess.ExitCode -ne 122 -or -not $canaryErr.Contains('ERR:E-SH-IR-STATEMENT')) {
            throw "formatter canary did not reach the declared typed boundary: exit=$($canaryProcess.ExitCode) stderr=$canaryErr stdout=$canaryOut"
        }
        if ($canaryErr.Contains('"class":"trap"')) { throw 'formatter canary regressed to a guest trap' }
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

Write-Output 'STEP_0250_OK bytes-params=lowering bytes-while=probed canary=STATEMENT next=if-else-regions'
