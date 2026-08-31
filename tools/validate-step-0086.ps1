param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
. (Join-Path $root 'tools\lib\native-command.ps1')

Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked', '-p', 'sico-ir', '-p', 'sico-semantics',
    '-p', 'sico-codegen-wasm', '-p', 'sico-cli'
) 'STEP-0086 workspace tests failed'

$runnerDir = Join-Path $root 'runner\sico-runner'
Invoke-NativeChecked $cargo @(
    'build', '--release', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml')
) 'sico-runner build failed'

Invoke-NativeChecked $cargo @('build', '-q', '--offline', '--locked', '-p', 'sico-cli') 'sico build failed'
$sico = Join-Path $root 'target\debug\sico.exe'
$env:SICO_RUNNER = Join-Path $runnerDir 'target\release\sico-runner.exe'
$work = Join-Path $root 'target\evidence\step-0086'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force
$env:SICO_CACHE_DIR = Join-Path $work 'cache'

# --- read-once: streaming handle + chunk into the buffered result ---
$readOnce = Join-Path $root 'tests\end-to-end\script-stream-read-once.sico'
$ro = 'hello stream' | & $sico run $readOnce 2>$work\r1e.txt
if ($LASTEXITCODE -ne 0 -or ($ro -join '') -cne 'hello stream') { throw "read-once failed: $($ro -join '')" }

# --- guest pump: 1 MiB and 16 MiB chunked passthrough ---
$pt = Join-Path $root 'tests\end-to-end\script-stream-passthrough.sico'
[IO.File]::WriteAllBytes((Join-Path $work 'in1.bin'), [byte[]]::new(1048576))
[IO.File]::WriteAllBytes((Join-Path $work 'in16.bin'), [byte[]]::new(16777216))
$null = & cmd.exe /c "`"$sico`" run `"$pt`" < `"$work\in1.bin`" > `"$work\p1.bin`" 2>NUL"
if ((Get-Item (Join-Path $work 'p1.bin')).Length -ne 1048576) { throw '1 MiB guest pump moved wrong byte count' }
$null = & cmd.exe /c "`"$sico`" run `"$pt`" < `"$work\in16.bin`" > `"$work\p16.bin`" 2>NUL"
if ((Get-Item (Join-Path $work 'p16.bin')).Length -ne 16777216) { throw '16 MiB guest pump moved wrong byte count' }

# --- host pump: 256 MiB passthrough at bounded RSS ---
$pump = Join-Path $root 'tests\end-to-end\script-stream-pump.sico'
[IO.File]::WriteAllBytes((Join-Path $work 'in256.bin'), [byte[]]::new(268435456))
$pumpComponent = Join-Path $work 'pump.component.wasm'
& $sico build --profile script-v0 -o $pumpComponent $pump | Out-Null
if ($LASTEXITCODE -ne 0) { throw '256 MiB pump Component build failed' }
$pumpError = Join-Path $work 'p256.err'
$pumpProcess = Start-Process -FilePath $env:SICO_RUNNER -ArgumentList @($pumpComponent) `
    -RedirectStandardInput (Join-Path $work 'in256.bin') `
    -RedirectStandardOutput (Join-Path $work 'p256.bin') `
    -RedirectStandardError $pumpError -PassThru -WindowStyle Hidden
$peakRss = 0L
while (-not $pumpProcess.HasExited) {
    $pumpProcess.Refresh()
    $peakRss = [Math]::Max($peakRss, $pumpProcess.WorkingSet64)
    Start-Sleep -Milliseconds 5
}
$pumpProcess.WaitForExit()
$pumpProcess.Refresh()
$pumpExit = [int]$pumpProcess.ExitCode
if ($pumpExit -ne 0) { throw "256 MiB host pump failed with exit $pumpExit`: $(Get-Content $pumpError -Raw)" }
if ((Get-Item (Join-Path $work 'p256.bin')).Length -ne 268435456) { throw '256 MiB host pump moved wrong byte count' }
if ($peakRss -gt 134217728) { throw "256 MiB host pump peak RSS exceeded 128 MiB: $peakRss" }
"peak_rss_bytes=$peakRss" | Out-File (Join-Path $work 'rss.txt') -Encoding ascii

# --- use-after-close: stale handle rejected with a trap (fail closed) ---
$uac = Join-Path $root 'tests\end-to-end\script-stream-use-after-close.sico'
$ua = 'AB' | & $sico run $uac 2>&1
if ($LASTEXITCODE -ne 125 -or ($ua -join '') -notmatch '"trap"') { throw "use-after-close must trap with 125, got $LASTEXITCODE" }

# --- mixing rule: a streams component leaves the buffered stdin empty ---
$mixOut = 'buffered-or-streamed' | & $sico run $readOnce 2>$work\r2e.txt
if ($LASTEXITCODE -ne 0 -or ($mixOut -join '') -cne 'buffered-or-streamed') { throw 'mixing rule: stream read must see the OS stdin' }

Write-Output "STEP_0086_OK read-once pump=1MiB,16MiB host-pump=256MiB peak-rss=$peakRss use-after-close=125 mixing-rule"
