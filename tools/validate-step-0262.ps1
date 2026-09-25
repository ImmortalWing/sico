param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$compilerTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8

foreach ($marker in @(
    'function gw_text_concat_packed(',
    'let gcc_cell_idx = local_binding_index(cell_names, word_at(words, cond_idx))',
    'let gtc_sub_pack = gw_text_concat_packed(',
    'let general_fallback_json = general_while_function_ir(',
    'let general_fallback_needed = I64.literal(0)',
    'SKIP:CONTROL-IF-DECLINED',
    'if U64.equal(if_count, U64.literal(0)):',
    '"u64", "sico.list.length")',
    'sico_compiler_lowers_the_format_code_region_byte_exactly'
)) {
    if (-not $parser.Contains($marker)) {
        if (-not $compilerTest.Contains($marker)) { throw "missing STEP-0262 marker: $marker" }
    }
}

# The retired frontier guards are gone.
if ($parser.Contains('SKIP:GENERAL-WHILE-ENTRYIF')) {
    throw 'stale SKIP:GENERAL-WHILE-ENTRYIF guard still present in parser.sico'
}
if ($parser.Contains('SKIP:GENERAL-WHILE-NOWHILE')) {
    throw 'stale SKIP:GENERAL-WHILE-NOWHILE guard still present in parser.sico'
}

# Refusal identity must not carry diagnostic suffixes.
foreach ($suffix in @('-L', '-E1', '-D0', '-RS')) {
    if ($parser.Contains("STATEMENT$suffix") -or $parser.Contains("PARAMETERS$suffix")) {
        throw "diagnostic suffix $suffix leaked back into parser.sico"
    }
}

Push-Location $root
try {
    # GNU toolchain + repo-local MSYS2 binutils (STEP-0261 host note).
    $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
    $binutils = Join-Path $root 'target\tooling\msys2-binutils\mingw64\bin'
    if (Test-Path -LiteralPath $binutils -PathType Container) {
        $env:Path = "$binutils;$env:Path"
    }
    & $CargoPath build --locked --offline -p sico-cli
    if ($LASTEXITCODE -ne 0) { throw 'sico CLI build failed' }
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host compiler differential failed' }
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host parser differential failed' }
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_local_bounds -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host link unit local bounds regression failed' }

    $sico = Join-Path $root 'target\debug\sico.exe'
    $runner = Join-Path $root 'runner\sico-runner\target\debug\sico-runner.exe'
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $temp = Join-Path $tempRoot ("sico-step0262-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $temp | Out-Null
    try {
        $component = Join-Path $temp 'compiler.component.wasm'
        & $sico build --profile script-v0 --output $component (Join-Path $root 'selfhost\compiler.sico')
        if ($LASTEXITCODE -ne 0) { throw 'self-host compiler bootstrap build failed' }

        $startInfo = [Diagnostics.ProcessStartInfo]::new()
        $startInfo.FileName = $runner
        $startInfo.Arguments = "--fuel 5000000000 `"$component`""
        $startInfo.WorkingDirectory = $temp
        $startInfo.RedirectStandardInput = $true
        $startInfo.RedirectStandardOutput = $true
        $startInfo.RedirectStandardError = $true
        $startInfo.UseShellExecute = $false
        $process = [Diagnostics.Process]::Start($startInfo)

        # Twenty-two-function formatter prefix (through repeat_indent:
        # call_left, format_code and repeat_indent land in this step) must be
        # byte-identical to the Rust canonical IR dump frozen by this step.
        $full = [IO.File]::ReadAllText((Join-Path $root 'selfhost\formatter.sico'))
        $boundary = $full.IndexOf('function nearest_match')
        if ($boundary -lt 0) { throw 'formatter boundary marker missing' }
        $prefix = Join-Path $temp 'formatter-prefix.sico'
        [IO.File]::WriteAllText($prefix, $full.Substring(0, $boundary))
        $stdinBytes = [Text.UTF8Encoding]::new($false).GetBytes($full.Substring(0, $boundary))
        $process.StandardInput.BaseStream.Write($stdinBytes, 0, $stdinBytes.Length)
        $process.StandardInput.BaseStream.Flush()
        $process.StandardInput.BaseStream.Close()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "formatter prefix did not compile: exit=$($process.ExitCode) stderr=$stderr" }
        $expectedPath = Join-Path $PSScriptRoot 'fixtures\step-0262\fc_expected.json'
        if (Test-Path $expectedPath) {
            $got = $stdout | ConvertFrom-Json
            $expected = (Get-Content -LiteralPath $expectedPath -Raw -Encoding UTF8) | ConvertFrom-Json
            if (($got | ConvertTo-Json -Depth 64 -Compress) -ne ($expected | ConvertTo-Json -Depth 64 -Compress)) {
                throw 'formatter prefix IR diverged from the Rust canonical dump'
            }
        }

        # STEP-0276 absorbed the Map-parameter WIP. The current frontier
        # remains nearest_match, now at GWPACK-OTHER rather than UNRESOLVED.
        $boundary2 = $full.IndexOf('function set_nearest_match')
        if ($boundary2 -lt 0) { throw 'nearest_match boundary marker missing' }
        $probe = Join-Path $temp 'nearest-match-probe.sico'
        [IO.File]::WriteAllText($probe, $full.Substring(0, $boundary2))
        $process1 = [Diagnostics.Process]::Start($startInfo)
        $probeBytes = [Text.UTF8Encoding]::new($false).GetBytes($full.Substring(0, $boundary2))
        $process1.StandardInput.BaseStream.Write($probeBytes, 0, $probeBytes.Length)
        $process1.StandardInput.BaseStream.Flush()
        $process1.StandardInput.BaseStream.Close()
        $null = $process1.StandardOutput.ReadToEnd()
        $stderr1 = $process1.StandardError.ReadToEnd()
        $process1.WaitForExit()
        if ($process1.ExitCode -ne 122 -or -not $stderr1.Contains('ERR:E-SH-IR-GWPACK-OTHER')) {
            throw "nearest_match frontier probe changed: exit=$($process1.ExitCode) stderr=$stderr1"
        }

        # Full-source canary: fail-closed at the declared nearest_match boundary.
        $process2 = [Diagnostics.Process]::Start($startInfo)
        $stdinBytes2 = [Text.UTF8Encoding]::new($false).GetBytes($full)
        $process2.StandardInput.BaseStream.Write($stdinBytes2, 0, $stdinBytes2.Length)
        $process2.StandardInput.BaseStream.Flush()
        $process2.StandardInput.BaseStream.Close()
        $null = $process2.StandardOutput.ReadToEnd()
        $stderr2 = $process2.StandardError.ReadToEnd()
        $process2.WaitForExit()
        if ($process2.ExitCode -ne 122 -or -not $stderr2.Contains('ERR:E-SH-IR-GWPACK-OTHER')) {
            throw "formatter canary did not reach the declared nearest_match boundary: exit=$($process2.ExitCode) stderr=$stderr2"
        }
        if ($stderr2.Contains('"class":"trap"')) { throw 'formatter canary regressed to a guest trap' }
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

Write-Output 'STEP_0262_OK call-left+format-code+repeat-indent=byte-exact parity=22-functions canary=NEAREST-MATCH-GWPACK-OTHER repinned-by=STEP-0280'
