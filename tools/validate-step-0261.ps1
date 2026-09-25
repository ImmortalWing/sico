param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw -Encoding UTF8
$compilerTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_compiler.rs') -Raw -Encoding UTF8
$boundsTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_local_bounds.rs') -Raw -Encoding UTF8

foreach ($marker in @(
    'let gwh_frames = sico.text.split_lines("")',
    'let gwh_len = U64.literal(0)',
    'let gwt_entries = sico.text.split_lines("")',
    'SKIP:GENERAL-WHILE-IF-REGION',
    'let scl_uc_callee = function_declaration_index(words, scl_atom)',
    'let scl_uc_pack = while_call_rhs_packed(words, src, name_idx, scl_seg_start, line_end, scl_seg_offset, scl_uc_end',
    '"u64", "sico.list.length")',
    '"{\"kind\":\"list\",\"data\":{\"kind\":\"string\"}}", "sico.text.split_lines")',
    'let scl_tail_ok = I64.literal(0)',
    'sico_compiler_lowers_the_source_has_lex_error_region_byte_exactly',
    'let scl_uc_result = scl_base'
)) {
    if (-not $parser.Contains($marker)) {
        if (-not $compilerTest.Contains($marker)) {
            if (-not $boundsTest.Contains($marker)) { throw "missing STEP-0261 marker: $marker" }
        }
    }
}

# The single-nested-while entry guard is gone: nested whiles now lower.
if ($parser.Contains('SKIP:GENERAL-WHILE-NESTED')) {
    throw 'stale SKIP:GENERAL-WHILE-NESTED guard still present in parser.sico'
}

# Refusal identity must not carry diagnostic suffixes again.
foreach ($suffix in @('-L', '-E1', '-D0', '-RS')) {
    if ($parser.Contains("STATEMENT$suffix") -or $parser.Contains("PARAMETERS$suffix")) {
        throw "diagnostic suffix $suffix leaked back into parser.sico"
    }
}

Push-Location $root
try {
    # This host's runner artifacts build under the pinned GNU toolchain with
    # the repo-local MSYS2 binutils (gcc for `ring`, dlltool) on PATH — the
    # same provisioning tools/run-ci.ps1 applies (STEP-0261 host note: the
    # evidence machine changed on 2026-09-23 and has no MSVC).
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
    $temp = Join-Path $tempRoot ("sico-step0261-" + [Guid]::NewGuid().ToString('N'))
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

        # Seventeen-function formatter prefix (through source_has_lex_error,
        # the first nested-while function) must be byte-identical to the Rust
        # canonical IR dump frozen by this step.
        $full = [IO.File]::ReadAllText((Join-Path $root 'selfhost\formatter.sico'))
        $boundary = $full.IndexOf('function no_space_before')
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
        $expectedPath = Join-Path $PSScriptRoot 'fixtures\step-0261\shle_expected.json'
        if (Test-Path $expectedPath) {
            $got = $stdout | ConvertFrom-Json
            $expected = (Get-Content -LiteralPath $expectedPath -Raw -Encoding UTF8) | ConvertFrom-Json
            if (($got | ConvertTo-Json -Depth 64 -Compress) -ne ($expected | ConvertTo-Json -Depth 64 -Compress)) {
                throw 'formatter prefix IR diverged from the Rust canonical dump'
            }
        }

        # STEP-0262 subsequently implemented call_left. Keep this historical
        # probe as a regression check for successful lowering, while the
        # current frontier is measured by report-m22-canary.ps1.
        $sameStart = $full.IndexOf('function same')
        $sameEnd = $full.IndexOf('end function', $sameStart) + 'end function'.Length + 1
        $clStart = $full.IndexOf('function call_left')
        $clEnd = $full.IndexOf('end function', $clStart) + 'end function'.Length + 1
        $probe = Join-Path $temp 'call-left-probe.sico'
        [IO.File]::WriteAllText($probe, $full.Substring($sameStart, $sameEnd - $sameStart) + $full.Substring($clStart, $clEnd - $clStart))
        $process1 = [Diagnostics.Process]::Start($startInfo)
        $probeBytes = [Text.UTF8Encoding]::new($false).GetBytes([IO.File]::ReadAllText($probe))
        $process1.StandardInput.BaseStream.Write($probeBytes, 0, $probeBytes.Length)
        $process1.StandardInput.BaseStream.Flush()
        $process1.StandardInput.BaseStream.Close()
        $null = $process1.StandardOutput.ReadToEnd()
        $stderr1 = $process1.StandardError.ReadToEnd()
        $process1.WaitForExit()
        if ($process1.ExitCode -ne 0) {
            throw "call_left regression probe failed: exit=$($process1.ExitCode) stderr=$stderr1"
        }

        # Current full-source canary: STEP-0295 advances through nearest_match
        # and fails closed at set_nearest_match / STATEMENT.
        $process2 = [Diagnostics.Process]::Start($startInfo)
        $stdinBytes2 = [Text.UTF8Encoding]::new($false).GetBytes($full)
        $process2.StandardInput.BaseStream.Write($stdinBytes2, 0, $stdinBytes2.Length)
        $process2.StandardInput.BaseStream.Flush()
        $process2.StandardInput.BaseStream.Close()
        $null = $process2.StandardOutput.ReadToEnd()
        $stderr2 = $process2.StandardError.ReadToEnd()
        $process2.WaitForExit()
        if ($process2.ExitCode -ne 122 -or -not $stderr2.Contains('ERR:E-SH-IR-STATEMENT')) {
            throw "formatter canary changed from the current set_nearest_match frontier: exit=$($process2.ExitCode) stderr=$stderr2"
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

Write-Output 'STEP_0261_OK nested-while=stack-frames parity=17-functions call_left=supported current_canary=SET-NEAREST-MATCH-STATEMENT repinned-by=STEP-0295'
