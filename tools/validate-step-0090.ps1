param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
. (Join-Path $root 'tools\lib\native-command.ps1')

Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-cli') 'STEP-0090 CLI tests failed'
Invoke-NativeChecked $cargo @('build', '--offline', '--locked', '-p', 'sico-cli') 'sico build failed'

$runnerDir = Join-Path $root 'runner\sico-runner'
$runnerManifest = Join-Path $runnerDir 'Cargo.toml'
$runnerOutput = (Invoke-NativeChecked $cargo @(
    'test', '--release', '--offline', '--locked', '--manifest-path', $runnerManifest,
    'prepared_program_reruns_are_isolated_and_survive_a_trap', '--', '--nocapture'
) 'persistent runner release test failed' 2>&1) -join "`n"
Invoke-NativeChecked $cargo @('build', '--release', '--offline', '--locked', '--manifest-path', $runnerManifest) 'persistent runner release build failed' | Out-Null
Write-Output $runnerOutput
$medianMatch = [regex]::Match($runnerOutput, 'PERSISTENT_WARM_MEDIAN_US=(\d+)')
if (-not $medianMatch.Success) { throw 'persistent warm median evidence missing' }
$medianUs = [int64]$medianMatch.Groups[1].Value
if ($medianUs -gt 20000) { throw "persistent warm median exceeds 20 ms: ${medianUs}us" }

$runId = [Guid]::NewGuid().ToString('N')
$work = Join-Path $root "target\evidence\step-0090\$runId"
New-Item -ItemType Directory -Force -Path $work | Out-Null
$source = Join-Path $work 'watch.sico'
$echoText = Get-Content (Join-Path $root 'tests\end-to-end\script-args-echo.sico') -Raw -Encoding UTF8
$rejectText = Get-Content (Join-Path $root 'tests\end-to-end\script-reject.sico') -Raw -Encoding UTF8
[IO.File]::WriteAllText($source, $echoText, [Text.UTF8Encoding]::new($false))

$sico = Join-Path $root 'target\debug\sico.exe'
$runner = Join-Path $runnerDir 'target\release\sico-runner.exe'
$psi = [Diagnostics.ProcessStartInfo]::new()
$psi.FileName = $sico
$psi.Arguments = "watch --max-runs 3 --poll-ms 10 `"$source`" -- generation"
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.EnvironmentVariables['SICO_RUNNER'] = $runner
$process = [Diagnostics.Process]::Start($psi)
$stdoutTask = $process.StandardOutput.ReadToEndAsync()
$stderrTask = $process.StandardError.ReadToEndAsync()

# Initial generation, then one invalid edit that must never reach the runner.
Start-Sleep -Milliseconds 1200
[IO.File]::WriteAllText($source, 'not valid sico source', [Text.UTF8Encoding]::new($false))
Start-Sleep -Milliseconds 400

# A typed domain failure must not poison the persistent process.
[IO.File]::WriteAllText($source, $rejectText, [Text.UTF8Encoding]::new($false))
Start-Sleep -Milliseconds 800

# Two rapid successful edits coalesce into one accepted generation.
$markerOne = $echoText + "`nfunction watch_marker() returns Int:`n  return 1`nend function`n"
$markerTwo = $echoText + "`nfunction watch_marker() returns Int:`n  return 2`nend function`n"
[IO.File]::WriteAllText($source, $markerOne, [Text.UTF8Encoding]::new($false))
Start-Sleep -Milliseconds 20
[IO.File]::WriteAllText($source, $markerTwo, [Text.UTF8Encoding]::new($false))

if (-not $process.WaitForExit(10000)) {
    & taskkill.exe /PID $process.Id /T /F | Out-Null
    $process.WaitForExit()
    throw 'sico watch did not finish three accepted generations'
}
$stdout = $stdoutTask.GetAwaiter().GetResult()
$stderr = $stderrTask.GetAwaiter().GetResult()
if ($process.ExitCode -ne 0) { throw "sico watch final exit failed: $($process.ExitCode) $stderr" }
if ($stdout -cne 'generationgeneration') { throw "watch output generations mismatch: $stdout" }

$events = @()
foreach ($line in ($stderr -split "`r?`n")) {
    try {
        $record = $line | ConvertFrom-Json -ErrorAction Stop
        if ($record.schema -eq 'sico.runner.watch.v0' -and $record.event -eq 'run-complete') {
            $events += $record
        }
    } catch {
        # Compiler diagnostics are intentionally ordinary stderr text in v0.
    }
}
if ($events.Count -ne 3) { throw "expected exactly 3 coalesced run events, got $($events.Count): $stderr" }
if (($events.generation -join ',') -ne '1,2,3') { throw 'watch generations are not monotonic' }
if (($events.exit -join ',') -ne '0,122,0') { throw "watch failure isolation mismatch: $($events.exit -join ',')" }
if ((@($events.pid | Select-Object -Unique)).Count -ne 1) { throw 'watch replaced the persistent runner process' }

$watchArtifacts = @(Get-ChildItem ([IO.Path]::GetTempPath()) -Filter "sico-watch-$($process.Id)-*.component.wasm" -ErrorAction SilentlyContinue)
if ($watchArtifacts.Count -ne 0) { throw 'sico watch leaked its temporary Component artifact' }

Write-Output ("STEP_0090_OK pid={0} generations=1,2,3 exits=0,122,0 invalid=not-run rapid-edits=coalesced warm-median={1}us artifact=clean work={2}" -f $events[0].pid, $medianUs, $work)
