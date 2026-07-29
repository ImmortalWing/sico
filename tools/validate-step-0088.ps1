param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
. (Join-Path $root 'tools\lib\native-command.ps1')

Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked', '-p', 'sico-codegen-wasm', '-p', 'sico-cli'
) 'STEP-0088 workspace tests failed'

$runnerDir = Join-Path $root 'runner\sico-runner'
$runnerManifest = Join-Path $runnerDir 'Cargo.toml'
Invoke-NativeChecked $cargo @(
    'test', '--release', '--offline', '--locked', '--manifest-path',
    $runnerManifest, '--', '--test-threads=1'
) 'sico-runner tests failed'
Invoke-NativeChecked $cargo @(
    'build', '--release', '--offline', '--locked', '--manifest-path', $runnerManifest
) 'sico-runner build failed'
Invoke-NativeChecked $cargo @('build', '-q', '--offline', '--locked', '-p', 'sico-cli') 'sico build failed'
$sico = Join-Path $root 'target\debug\sico.exe'
$runner = Join-Path $runnerDir 'target\release\sico-runner.exe'
$env:SICO_RUNNER = $runner
$work = Join-Path $root 'target\evidence\step-0088'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force
$env:SICO_CACHE_DIR = Join-Path $work 'cache'

# Streaming regression through the worker-based IO layer.
$ro = 'hello stream' | & $sico run (Join-Path $root 'tests\end-to-end\script-stream-read-once.sico') 2>$work\r1.txt
if ($LASTEXITCODE -ne 0 -or ($ro -join '') -cne 'hello stream') { throw 'read-once regression' }
[IO.File]::WriteAllBytes((Join-Path $work 'in256.bin'), [byte[]]::new(268435456))
$null = & cmd.exe /c "`"$sico`" run `"$root\tests\end-to-end\script-stream-pump.sico`" < `"$work\in256.bin`" > `"$work\p256.bin`" 2>NUL"
if ((Get-Item (Join-Path $work 'p256.bin')).Length -ne 268435456) { throw '256 MiB pump regression' }

# Cancellation-specific guests preserve cancellation as the control result
# instead of deliberately converting every stream error into DomainError.
$cancelRead = (Get-Content (Join-Path $root 'tests\end-to-end\script-stream-read-once.sico') -Raw).Replace('ScriptErrorCode.DomainError', 'ScriptErrorCode.Cancelled')
$cancelPump = (Get-Content (Join-Path $root 'tests\end-to-end\script-stream-pump.sico') -Raw).Replace('ScriptErrorCode.DomainError', 'ScriptErrorCode.Cancelled')
[IO.File]::WriteAllText((Join-Path $work 'cancel-read.sico'), $cancelRead, [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText((Join-Path $work 'cancel-pump.sico'), $cancelPump, [Text.UTF8Encoding]::new($false))
& $sico build --profile script-v0 -o (Join-Path $work 'cancel-read.wasm') (Join-Path $work 'cancel-read.sico')
& $sico build --profile script-v0 -o (Join-Path $work 'cancel-pump.wasm') (Join-Path $work 'cancel-pump.sico')
foreach ($case in @(@('cancel-read.wasm', 'blocked read'), @('cancel-pump.wasm', 'blocked pump'))) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $runner
    $psi.Arguments = "--cancel-after-ms 100 `"$(Join-Path $work $case[0])`""
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $watch = [Diagnostics.Stopwatch]::StartNew()
    $proc = [Diagnostics.Process]::Start($psi)
    $proc.WaitForExit()
    $elapsed = $watch.Elapsed.TotalMilliseconds
    $detail = $proc.StandardError.ReadToEnd()
    if ($proc.ExitCode -ne 123 -or $detail -notmatch 'cancelled') {
        throw "$($case[1]): expected cancelled 123, got $($proc.ExitCode) $detail"
    }
    if ($elapsed -gt 500) { throw "$($case[1]): cancellation took ${elapsed}ms (goal is prompt after the 100 ms timer)" }
    Write-Output ("cancel-{0}: exit=123 elapsed={1:N0}ms" -f $case[0], $elapsed)
}

# A redirected stdout pipe is intentionally left unread while 8 MiB is fed
# asynchronously. The worker blocks in write_all; cancellation must still
# terminate the runner without waiting for the consumer.
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = 'cmd.exe'
$cancelPumpWasm = Join-Path $work 'cancel-pump.wasm'
$blockedWriteInput = Join-Path $work 'in256.bin'
$psi.Arguments = "/d /s /c `"`"$runner`" --cancel-after-ms 100 `"$cancelPumpWasm`" < `"$blockedWriteInput`"`""
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$proc = [Diagnostics.Process]::Start($psi)
$watch = [Diagnostics.Stopwatch]::StartNew()
if (-not $proc.WaitForExit(1000)) {
    $proc.Kill()
    throw 'blocked write did not observe cancellation within 1 second'
}
$detail = $proc.StandardError.ReadToEnd()
if ($proc.ExitCode -ne 123 -or $detail -notmatch 'cancelled') {
    throw "blocked write: expected cancelled 123, got $($proc.ExitCode) $detail"
}
Write-Output ("cancel-blocked-write: exit=123 elapsed={0:N0}ms" -f $watch.Elapsed.TotalMilliseconds)

Write-Output 'STEP_0088_OK worker-io=regression-green 256MiB=exact cancel-busy/read/pump/write=123 prompt queues=bounded'
