param(
    [string]$ComponentPath = '',
    [switch]$SkipBuild
)

# R1 canary coverage report (M22-26 route replan v1 §3 R1).
# Measures, by execution against the current tree:
#   - functions_total / functions_covered of selfhost/formatter.sico (the
#     declared canary unit: a function counts as covered when the guest
#     compiler lowers the source prefix THROUGH that function with exit 0);
#   - the full-source typed refusal frontier (function + identity).
# Output: one JSON object (schema sico.m22.canary.v0) on stdout.
# Evidence note: this host reuses prebuilt debug artifacts when -SkipBuild is
# given; rebuilds pin the GNU toolchain per the STEP-0261 host note.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$formatterPath = Join-Path $root 'selfhost\formatter.sico'
$full = [IO.File]::ReadAllText($formatterPath)
$formatterLines = ($full -split "`n").Count

# Function index: name -> (1-based source order, end offset in chars).
$matches = [regex]::Matches($full, '(?m)^function ([A-Za-z_][A-Za-z0-9_]*)')
$functions = @()
for ($i = 0; $i -lt $matches.Count; $i++) {
    $start = $matches[$i].Index
    $end = if ($i + 1 -lt $matches.Count) { $matches[$i + 1].Index } else { $full.Length }
    $functions += [pscustomobject]@{
        name = $matches[$i].Groups[1].Value
        order = $i + 1
        start = $start
        end = $end
    }
}
$total = $functions.Count
if ($total -lt 2) { throw 'formatter.sico function scan failed' }

# Component resolution.
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$temp = Join-Path $tempRoot ("sico-canary-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    if (-not $ComponentPath) {
        if ($SkipBuild) { throw '-SkipBuild requires -ComponentPath' }
        $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
        $binutils = Join-Path $root 'target\tooling\msys2-binutils\mingw64\bin'
        if (Test-Path -LiteralPath $binutils -PathType Container) {
            $env:Path = "$binutils;$env:Path"
        }
        $cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
        & $cargo build --locked --offline -p sico-cli
        if ($LASTEXITCODE -ne 0) { throw 'sico CLI build failed' }
        $ComponentPath = Join-Path $temp 'compiler.component.wasm'
        & (Join-Path $root 'target\debug\sico.exe') build --profile script-v0 --output $ComponentPath (Join-Path $root 'selfhost\compiler.sico')
        if ($LASTEXITCODE -ne 0) { throw 'self-host compiler component build failed' }
    }
    $runner = Join-Path $root 'runner\sico-runner\target\debug\sico-runner.exe'
    $cap = 5000000000

    $runGuest = {
        param($sourceText)
        $psi = [Diagnostics.ProcessStartInfo]::new()
        $psi.FileName = $runner
        $psi.Arguments = "--fuel $cap `"$ComponentPath`""
        $psi.WorkingDirectory = $temp
        $psi.RedirectStandardInput = $true
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.UseShellExecute = $false
        $p = [Diagnostics.Process]::Start($psi)
        $bytes = [Text.UTF8Encoding]::new($false).GetBytes($sourceText)
        try {
            $p.StandardInput.BaseStream.Write($bytes, 0, $bytes.Length)
            $p.StandardInput.BaseStream.Flush()
            $p.StandardInput.BaseStream.Close()
        }
        catch {
            $earlyErr = ''
            try { $earlyErr = $p.StandardError.ReadToEnd() } catch {}
            throw "guest stdin write failed (early-exit=$($p.HasExited)): $earlyErr"
        }
        $stdout = $p.StandardOutput.ReadToEnd()
        $stderr = $p.StandardError.ReadToEnd()
        $p.WaitForExit()
        return [pscustomobject]@{ exit = $p.ExitCode; stderr = $stderr; stdout = $stdout }
    }

    # Full-source frontier.
    $fullRun = & $runGuest $full
    $frontierCode = ''
    $m = [regex]::Match($fullRun.stderr, 'ERR:[A-Z0-9-]+')
    if ($m.Success) { $frontierCode = $m.Value }

    # Binary search the largest covered prefix: functions are contiguous from
    # the top; a prefix through function order k succeeds with exit 0.
    $lo = 0  # functions[0..lo-1] known-good (none yet)
    $hi = $total  # invariant: prefix through $hi fails or is untested
    # Establish the invariant: if the whole file passes, everything is covered.
    if ($fullRun.exit -eq 0) {
        $lo = $total
    }
    else {
        while ($hi - $lo -gt 1) {
            $mid = [int](($lo + $hi) / 2)
            $prefix = $full.Substring(0, $functions[$mid - 1].end)
            $r = & $runGuest $prefix
            if ($r.exit -eq 0) { $lo = $mid } else { $hi = $mid }
        }
    }
    $covered = $lo
    $frontierFunction = if ($covered -lt $total) { $functions[$covered].name } else { '' }

    $report = [ordered]@{
        schema = 'sico.m22.canary.v0'
        generated_utc = (Get-Date).ToUniversalTime().ToString('o')
        formatter_lines = $formatterLines
        functions_total = $total
        functions_covered = $covered
        coverage_pct = [math]::Round(100.0 * $covered / $total, 1)
        frontier_function = $frontierFunction
        frontier_code = $frontierCode
        full_source_exit = $fullRun.exit
        function_list = ($functions | ForEach-Object { $_.name })
        evidence = [ordered]@{
            component_path = $ComponentPath
            fuel_cap = $cap
            host_note = 'replan-v1 R1 canary report; ABI variant per local build cache (STEP-0261 host note)'
        }
    }
    $report | ConvertTo-Json -Depth 6
}
finally {
    $resolvedTemp = [IO.Path]::GetFullPath($temp)
    if (-not $resolvedTemp.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing to remove non-temporary path: $resolvedTemp"
    }
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
}
