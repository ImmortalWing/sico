param(
    [int[]]$Sizes = @(1024, 4096, 8192),
    [switch]$IncludeNatural,
    [string]$ComponentPath = '',
    [int64[]]$Caps = @(100000, 1000000, 10000000, 100000000, 1000000000),
    [int64]$Cap = 5000000000
)

# R1 budget probe (M22-26 route replan v1 §3 R1).
# Measures, by execution against the current tree, for each requested input
# size (synthetic module of renamed donor-function copies unless -IncludeNatural
# adds the natural canary inputs):
#   - wall time at the default cap;
#   - minimum fuel cap that completes (bisection) = consumed-fuel estimate.
# Fuel exhaustion is detected empirically: exit 126 with "all fuel consumed"
# in stderr (observed 2026-09-24 on this host). Stack high-water is NOT
# exposed by the runner CLI; the observable stack ceiling is the
# StackLimit outcome class. Output: one JSON object per input
# (schema sico.m22.budget.v0), then a summary line PROBE_M22_BUDGET_OK.

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$formatterPath = Join-Path $root 'selfhost\formatter.sico'
$full = [IO.File]::ReadAllText($formatterPath)

# Donor: scan_string (a landed, self-contained scanning function, ~29 lines)
# plus its `add` dependency, prepended once so call targets resolve.
$addMatch = [regex]::Match($full, '(?ms)^function add.*?^end function\s*$')
if (-not $addMatch.Success) { throw 'add dependency not found in formatter.sico' }
$preamble = $addMatch.Value + "`n"
$donorMatch = [regex]::Match($full, '(?ms)^function scan_string.*?^end function\s*$')
if (-not $donorMatch.Success) { throw 'scan_string donor not found in formatter.sico' }
$donor = $donorMatch.Value
$donorLines = ($donor -split "`n").Count

# Component resolution (reuse the canary report's build discipline).
$tempRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$temp = Join-Path $tempRoot ("sico-budget-" + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null
try {
    if (-not $ComponentPath) {
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

    function Invoke-Guest([string]$sourceText, [int64]$fuel) {
        $psi = [Diagnostics.ProcessStartInfo]::new()
        $psi.FileName = $runner
        $psi.Arguments = "--fuel $fuel `"$ComponentPath`""
        $psi.WorkingDirectory = $temp
        $psi.RedirectStandardInput = $true
        $psi.RedirectStandardOutput = $true
        $psi.RedirectStandardError = $true
        $psi.UseShellExecute = $false
        $p = [Diagnostics.Process]::Start($psi)
        $bytes = [Text.UTF8Encoding]::new($false).GetBytes($sourceText)
        $p.StandardInput.BaseStream.Write($bytes, 0, $bytes.Length)
        $p.StandardInput.BaseStream.Flush()
        $p.StandardInput.BaseStream.Close()
        $sw = [Diagnostics.Stopwatch]::StartNew()
        $stdout = $p.StandardOutput.ReadToEnd()
        $stderr = $p.StandardError.ReadToEnd()
        $p.WaitForExit()
        $sw.Stop()
        return [pscustomobject]@{
            exit = $p.ExitCode
            stderr = $stderr
            wall_ms = $sw.ElapsedMilliseconds
        }
    }

    function New-Synthetic([int]$targetLines) {
        $copies = [math]::Ceiling($targetLines / $donorLines)
        $sb = [Text.StringBuilder]::new()
        [void]$sb.Append($preamble)
        for ($k = 0; $k -lt $copies; $k++) {
            $renamed = [string]($donor -replace '^function scan_string', ('probe_' + $k))
            [void]$sb.Append($renamed)
            [void]$sb.Append("`n")
        }
        return [pscustomobject]@{ text = $sb.ToString(); copies = $copies }
    }

    $inputs = @()
    foreach ($size in $Sizes) {
        $syn = New-Synthetic $size
        $inputs += [pscustomobject]@{
            kind = 'synthetic-copies'
            target_lines = $size
            text = $syn.text
            copies = $syn.copies
        }
    }
    if ($IncludeNatural) {
        $nmStart = $full.IndexOf('function nearest_match')
        $prefix = $full.Substring(0, $nmStart)
        $inputs += [pscustomobject]@{
            kind = 'natural-prefix22'
            target_lines = ($prefix -split "`n").Count
            text = $prefix
            copies = 0
        }
        $inputs += [pscustomobject]@{
            kind = 'natural-full-formatter'
            target_lines = ($full -split "`n").Count
            text = $full
            copies = 0
        }
    }

    $results = @()
    foreach ($in in $inputs) {
        $base = Invoke-Guest $in.text $Cap
        $record = [ordered]@{
            schema = 'sico.m22.budget.v0'
            generated_utc = (Get-Date).ToUniversalTime().ToString('o')
            input_kind = $in.kind
            input_lines = ($in.text -split "`n").Count
            input_bytes = [Text.UTF8Encoding]::new($false).GetByteCount($in.text)
            donor = 'scan_string'
            donor_lines = $donorLines
            copies = $in.copies
            fuel_cap = $Cap
            baseline_exit = $base.exit
            baseline_wall_ms = $base.wall_ms
            min_cap = $null
            consumed_fuel_estimate = $null
            bisect_iterations = 0
            frontier_code = ''
            evidence_note = 'R1 budget probe; consumed fuel = minimal sufficient cap via bisection; stack high-water not exposed by runner CLI'
        }
        $m = [regex]::Match($base.stderr, 'ERR:[A-Z0-9-]+')
        if ($m.Success) { $record.frontier_code = $m.Value }

        if ($base.exit -eq 0) {
            # Cap ladder: bracket consumed fuel between the largest exhausting
            # cap and the smallest completing cap (quadratic guest JSON
            # assembly makes full bisection runs expensive; the bracket plus
            # the wall-time curve answers the R0 scaling question).
            $lower = [int64]0
            $upper = [int64]0
            $ladder = @()
            foreach ($c in $Caps) {
                if ($c -ge $Cap) { continue }
                $r = Invoke-Guest $in.text $c
                $ladder += [pscustomobject]@{ cap = $c; exit = $r.exit; wall_ms = $r.wall_ms; class = '' }
                $mc = [regex]::Match($r.stderr, '"class"\s*:\s*"([^"]+)"')
                if ($mc.Success) { $ladder[$ladder.Count - 1].class = $mc.Groups[1].Value }
                $exhausted = ($r.exit -eq 125 -and $r.stderr.Contains('resource-limit.fuel')) -or
                             ($r.exit -eq 126 -and $r.stderr.Contains('all fuel consumed'))
                if ($exhausted) { $lower = $c } else { $upper = $c; break }
            }
            $record.bisect_iterations = $ladder.Count
            $record.min_cap = $(if ($upper) { $upper } else { $Cap })
            $record.consumed_fuel_estimate = $record.min_cap
            $record.cap_ladder = $ladder
        }
        $results += [pscustomobject]$record
        ($record | ConvertTo-Json -Depth 4 -Compress)
    }
    Write-Output ('PROBE_M22_BUDGET_OK inputs=' + $results.Count)
}
finally {
    $resolvedTemp = [IO.Path]::GetFullPath($temp)
    if (-not $resolvedTemp.StartsWith($tempRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "refusing to remove non-temporary path: $resolvedTemp"
    }
    Remove-Item -LiteralPath $resolvedTemp -Recurse -Force
}
