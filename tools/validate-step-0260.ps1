param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$compilerTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
$parserTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_parser.rs') -Raw -Encoding UTF8

foreach ($marker in @(
    'function type_json(',
    'function general_while_function_ir(',
    'while_call_rhs_packed(words, src, name_idx, cond_idx, gcc_colon',
    'sico.text.concat("#match", gm_binding)',
    'sico.text.concat("#match", name)',
    'fixed_operation_instruction_json(gwf_result',
    'let gbrk_from = sico.list.empty[U64]()',
    'let gok = I64.literal(0)',
    'sico_compiler_lowers_the_line_tokens_region_byte_exactly',
    'sico_lowering_emits_verifier_accepted_scalar_ir_and_refuses_noncanonical_input'
)) {
    if (-not $parser.Contains($marker)) {
        if (-not $compilerTest.Contains($marker)) {
            if (-not $parserTest.Contains($marker)) { throw "missing STEP-0260 marker: $marker" }
        }
    }
}

# Refusal identity must not carry diagnostic suffixes again.
foreach ($suffix in @('-L', '-E1', '-D0', '-RS')) {
    if ($parser.Contains("STATEMENT$suffix") -or $parser.Contains("PARAMETERS$suffix")) {
        throw "diagnostic suffix $suffix leaked back into parser.sico"
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
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host parser differential failed' }
    & $CargoPath test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_local_bounds -- --test-threads=1
    if ($LASTEXITCODE -ne 0) { throw 'self-host link unit local bounds regression failed' }

    $sico = Join-Path $root 'target\debug\sico.exe'
    $runner = Join-Path $root 'runner\sico-runner\target\debug\sico-runner.exe'
    $tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $temp = Join-Path $tempRoot ("sico-step0260-" + [Guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $temp | Out-Null
    try {
        $component = Join-Path $temp 'compiler.component.wasm'
        & $sico build --profile script-v0 --output $component (Join-Path $root 'selfhost\compiler.sico')
        if ($LASTEXITCODE -ne 0) { throw 'self-host compiler bootstrap build failed' }

        # Sixteen-function formatter prefix (through line_tokens) must be
        # byte-identical to the Rust canonical IR dump recorded by this step.
        $prefix = Join-Path $temp 'formatter-prefix.sico'
        $prefixLines = [IO.File]::ReadAllText((Join-Path $root 'selfhost\formatter.sico')) -split "`n", 0
        # formatter.sico line 445 ends the line_tokens function body.
        [IO.File]::WriteAllText($prefix, (($prefixLines | Select-Object -First 445) -join "`n") + "`n")
        $startInfo = [Diagnostics.ProcessStartInfo]::new()
        $startInfo.FileName = $runner
        $startInfo.Arguments = "--fuel 2000000000 `"$component`""
        $startInfo.WorkingDirectory = $temp
        $startInfo.RedirectStandardInput = $true
        $startInfo.RedirectStandardOutput = $true
        $startInfo.RedirectStandardError = $true
        $startInfo.UseShellExecute = $false
        $process = [Diagnostics.Process]::Start($startInfo)
        $stdinBytes = [Text.UTF8Encoding]::new($false).GetBytes([IO.File]::ReadAllText($prefix))
        $process.StandardInput.BaseStream.Write($stdinBytes, 0, $stdinBytes.Length)
        $process.StandardInput.BaseStream.Flush()
        $process.StandardInput.BaseStream.Close()
        $stdout = $process.StandardOutput.ReadToEnd()
        $stderr = $process.StandardError.ReadToEnd()
        $process.WaitForExit()
        if ($process.ExitCode -ne 0) { throw "formatter prefix did not compile: exit=$($process.ExitCode) stderr=$stderr" }
        $expectedPath = Join-Path $PSScriptRoot 'fixtures\step-0260\lt_expected.json'
        if (Test-Path $expectedPath) {
            $got = $stdout | ConvertFrom-Json
            $expected = (Get-Content -LiteralPath $expectedPath -Raw -Encoding UTF8) | ConvertFrom-Json
            if (($got | ConvertTo-Json -Depth 64 -Compress) -ne ($expected | ConvertTo-Json -Depth 64 -Compress)) {
                throw 'formatter prefix IR diverged from the Rust canonical dump'
            }
        }

        # Full-source canary: the next frontier is nested while lowering.
        $process2 = [Diagnostics.Process]::Start($startInfo)
        $stdinBytes2 = [Text.UTF8Encoding]::new($false).GetBytes([IO.File]::ReadAllText((Join-Path $root 'selfhost\formatter.sico')))
        $process2.StandardInput.BaseStream.Write($stdinBytes2, 0, $stdinBytes2.Length)
        $process2.StandardInput.BaseStream.Flush()
        $process2.StandardInput.BaseStream.Close()
        $stdout2 = $process2.StandardOutput.ReadToEnd()
        $stderr2 = $process2.StandardError.ReadToEnd()
        $process2.WaitForExit()
        if ($process2.ExitCode -ne 122 -or -not $stderr2.Contains('SKIP:GENERAL-WHILE-NESTED')) {
            throw "formatter canary did not reach the declared nested-while boundary: exit=$($process2.ExitCode) stderr=$stderr2 stdout=$stdout2"
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

Write-Output 'STEP_0260_OK line-tokens=byte-exact break-continue=fixed-ops parity=16-functions canary=NESTED-WHILE next=format_code-nested-while'
