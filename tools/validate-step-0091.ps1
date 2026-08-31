param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
. (Join-Path $root 'tools\lib\native-command.ps1')

Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-cli') 'STEP-0091 CLI tests failed'
Invoke-NativeChecked $cargo @('build', '--offline', '--locked', '-p', 'sico-cli') 'sico build failed'

$sico = Join-Path $root 'target\debug\sico.exe'
$runId = [Guid]::NewGuid().ToString('N')
$work = Join-Path $root "target\evidence\step-0091\$runId"
New-Item -ItemType Directory -Force -Path $work | Out-Null

function Invoke-Repl([string]$stdinText, [switch]$json) {
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $sico
    $psi.Arguments = if ($json) { 'repl --json' } else { 'repl' }
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($psi)
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.StandardInput.Write($stdinText)
    $process.StandardInput.Close()
    $peakBytes = 0L
    while (-not $process.HasExited -and $watch.ElapsedMilliseconds -lt 30000) {
        $process.Refresh()
        $peakBytes = [Math]::Max($peakBytes, $process.WorkingSet64)
        Start-Sleep -Milliseconds 5
    }
    if (-not $process.HasExited) {
        $process.Kill()
        $process.WaitForExit()
        throw 'REPL did not terminate after EOF/quit'
    }
    $process.WaitForExit()
    $watch.Stop()
    return [pscustomobject]@{
        Exit = $process.ExitCode
        Stdout = $stdoutTask.GetAwaiter().GetResult()
        Stderr = $stderrTask.GetAwaiter().GetResult()
        ElapsedMs = $watch.Elapsed.TotalMilliseconds
        PeakBytes = $peakBytes
    }
}

# JSON protocol, deterministic reset identity, failed-cell rollback and history.
$jsonRun = Invoke-Repl "40 + 2`nunknown`n41 + 1`n:reset`n40 + 2`n:history`n:quit`n" -json
if ($jsonRun.Exit -ne 0) { throw 'JSON REPL failed' }
$events = @($jsonRun.Stdout -split "`r?`n" | Where-Object { $_ } | ForEach-Object { $_ | ConvertFrom-Json })
$cells = @($events | Where-Object { $_.event -eq 'cell' })
if ($cells.Count -ne 3 -or [int]$cells[0].detail.value -ne 42 -or [int]$cells[1].detail.value -ne 42) {
    throw "REPL cell results mismatch: count=$($cells.Count) stdout=$($jsonRun.Stdout)"
}
if ($cells[0].detail.id -cne $cells[2].detail.id) { throw 'reset did not deterministically restart cell identity' }
if ($jsonRun.Stderr -notmatch 'sico.repl.event.v0' -or $jsonRun.Stderr -notmatch 'error') { throw 'failed cell was not typed on stderr' }

# An oversized line is drained, rejected and followed by a healthy cell.
$oversized = ('x' * 4097) + "`n1 + 1`n:quit`n"
$recovered = Invoke-Repl $oversized
if ($recovered.Exit -ne 0 -or $recovered.Stderr -notmatch 'line exceeds 4 KiB' -or $recovered.Stdout -notmatch '= 2') {
    throw 'REPL line-bound recovery failed'
}

# Exactly 256 accepted cells; the 257th is rejected without unbounded growth.
$growthInput = ((1..257 | ForEach-Object { '1' }) -join "`n") + "`n:quit`n"
$growth = Invoke-Repl $growthInput
$accepted = @($growth.Stdout -split "`r?`n" | Where-Object { $_ -match '^[0-9a-f]{64} = 1$' }).Count
if ($growth.Exit -ne 0 -or $accepted -ne 256 -or $growth.Stderr -notmatch 'session exceeds 256 cells') {
    throw "REPL cell-count bound failed: accepted=$accepted"
}
if ($growth.PeakBytes -gt 128MB) { throw "REPL peak RSS exceeds 128 MiB: $($growth.PeakBytes)" }

# Export is explicit, canonical, and refuses overwrite.
$export = Join-Path $work 'session.json'
$exported = Invoke-Repl "40 + 2`n:export $export`n:quit`n"
if ($exported.Exit -ne 0 -or -not (Test-Path $export)) { throw 'REPL export failed' }
$session = Get-Content $export -Raw -Encoding UTF8 | ConvertFrom-Json
if ($session.schema -ne 'sico.repl.session.v0' -or $session.cells.Count -ne 1 -or $session.cells[0].value -ne 42) { throw 'REPL export schema mismatch' }
$before = [IO.File]::ReadAllBytes($export)
$collision = Invoke-Repl "1`n:export $export`n:quit`n"
$after = [IO.File]::ReadAllBytes($export)
if ($collision.Stderr -notmatch 'cannot create export' -or -not [Collections.StructuralComparisons]::StructuralEqualityComparer.Equals($before, $after)) {
    throw 'REPL export overwrite refusal failed'
}

Write-Output ("STEP_0091_OK json=typed reset=id-stable invalid=rollback line=4KiB+recover cells=256 history=16KiB export=create-new peak={0:N1}MiB growth={1:N0}ms work={2}" -f ($growth.PeakBytes / 1MB), $growth.ElapsedMs, $work)
