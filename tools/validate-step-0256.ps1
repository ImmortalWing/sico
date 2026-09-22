param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$compilerTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8

foreach ($marker in @(
    'function list_get_match_ir(',
    'function nested_intrinsic_return_ir(',
    'function list_aware_parameter_index(',
    '"name\":\"sico.list.get\",\"arguments\":[',
    '"name\":\"sico.list.append\",\"arguments\":[',
    'sico_compiler_lowers_the_item_region_byte_exactly',
    'sico_compiler_lowers_the_append_pair_region_byte_exactly'
)) {
    if (-not $parser.Contains($marker)) {
        if (-not $compilerTest.Contains($marker)) { throw "missing STEP-0256 marker: $marker" }
    }
}

Push-Location $root
try {
    # This host's runner artifacts are built with the pinned MSVC toolchain
    # (STEP-0254 environment note); the GNU flow lacks a C compiler for
    # `ring` and msys2-binutils for dlltool on this machine.
    $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-msvc'
    & $CargoPath build --locked --offline -p sico-cli
    if ($LASTEXITCODE -ne 0) { throw 'sico CLI build failed' }
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host compiler differential failed' }
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_local_bounds -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host link unit local bounds regression failed' }

    $sico = Join-Path $root 'target\debug\sico.exe'
    $runner = Join-Path $root 'runner\sico-runner\target\debug\sico-runner.exe'
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $temp = Join-Path $tempRoot ("sico-step0256-" + [Guid]::NewGuid().ToString('N'))
    [void](New-Item -ItemType Directory -Path $temp)
    try {
        $compilerComponent = Join-Path $temp 'compiler.component.wasm'
        & $sico build --profile script-v0 --output $compilerComponent (Join-Path $root 'selfhost\compiler.sico') | Out-Null
        if ($LASTEXITCODE -ne 0) { throw 'self-host compiler component build failed' }

        $start = [Diagnostics.ProcessStartInfo]::new()
        $start.FileName = $runner
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
        # The formatter prefix through append_pair is byte-exact (fifteen
        # functions). The next function, line_tokens, interleaves while
        # bodies with multiple matches, which the general lowering still
        # typed-refuses (multi-match control).
        if ($process.ExitCode -ne 122 -or -not $stderr.Contains('ERR:E-SH-IR-CONTROL')) {
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

Write-Output 'STEP_0256_OK item=list-get-match-append-pair=nested-intrinsic parity=15-functions canary=CONTROL next=line-tokens-multi-match'
