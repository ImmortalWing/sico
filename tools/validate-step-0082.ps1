param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

cargo test --offline --locked -p sico-cli -p sico-package
if ($LASTEXITCODE -ne 0) { throw 'STEP-0082 workspace tests failed' }

$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { throw "MSVC vcvars64.bat not found: $vcvars" }
$runnerDir = Join-Path $root 'runner\sico-runner'
$build = "call `"$vcvars`" && set RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc&& cd /d `"$runnerDir`" && cargo build --release --offline"
$buildOutput = & cmd.exe /c $build
if ($LASTEXITCODE -ne 0) {
    $buildOutput | ForEach-Object { Write-Host $_ }
    throw 'sico-runner build failed'
}

cargo build -q --offline --locked -p sico-cli
if ($LASTEXITCODE -ne 0) { throw 'sico build failed' }
$sico = Join-Path $root 'target\debug\sico.exe'
$env:SICO_RUNNER = Join-Path $runnerDir 'target\release\sico-runner.exe'
$work = Join-Path $root 'target\evidence\step-0082'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force
$env:SICO_CACHE_DIR = Join-Path $work 'cache'
$echo = Join-Path $root 'tests\end-to-end\script-echo.sico'

$payload = 'step-0082-cache'
$first = $payload | & $sico run $echo 2>$work\e1.txt
if ($LASTEXITCODE -ne 0 -or ($first -join '') -cne $payload) { throw 'first run failed' }
$entry = Get-ChildItem (Join-Path $env:SICO_CACHE_DIR 'script-v0') -File | Select-Object -First 1
if (-not $entry) { throw 'cache entry was not created' }
$firstBytes = [IO.File]::ReadAllBytes($entry.FullName)
$firstWrite = $entry.LastWriteTimeUtc
Start-Sleep -Milliseconds 1100
$second = $payload | & $sico run $echo 2>$work\e2.txt
if ($LASTEXITCODE -ne 0 -or ($second -join '') -cne $payload) { throw 'second run failed' }
$entryAfter = Get-ChildItem (Join-Path $env:SICO_CACHE_DIR 'script-v0') -File
if ($entryAfter.Count -ne 1) { throw 'cache entry count changed on repeat run' }
if ([IO.File]::ReadAllBytes($entry.FullName).Length -ne $firstBytes.Length -or $entryAfter[0].LastWriteTimeUtc -ne $firstWrite) {
    throw 'cache entry was rewritten on repeat run'
}

# A changed source compiles to a fresh entry while the old one stays.
$changed = Join-Path $work 'changed.sico'
(Get-Content $echo -Raw) -replace 'exit_code: I64\.literal\(0\)', 'exit_code: I64.literal(7)' | Set-Content $changed -NoNewline
$null = $payload | & $sico run $changed 2>$work\e3.txt
$changedExit = $LASTEXITCODE
if ($changedExit -ne 7) { throw "changed-source run must exit 7, got $changedExit" }
$entries = Get-ChildItem (Join-Path $env:SICO_CACHE_DIR 'script-v0') -File
if ($entries.Count -ne 2) { throw 'changed source must create a fresh cache entry' }

# run - consumes source from stdin and never feeds it to the guest.
$dashOut = Get-Content $echo -Raw | & $sico run - 2>$work\e4.txt
if ($LASTEXITCODE -ne 0 -or ($dashOut -join '') -cne '') { throw 'run - must give the guest empty stdin' }

# eval: constant expressions only.
$evalOut = & $sico eval "40 + 2" 2>$null
if ($LASTEXITCODE -ne 0 -or ($evalOut -join '') -cne '42') { throw "eval must print 42, got $evalOut" }
$null = & $sico eval "input.stdin" 2>$null
if ($LASTEXITCODE -ne 120) { throw "non-constant eval must exit 120, got $LASTEXITCODE" }

# reject maps to 122; missing runner to 127.
$null = & $sico run (Join-Path $root 'tests\end-to-end\script-reject.sico') 2>$work\e5.txt
if ($LASTEXITCODE -ne 122) { throw "reject must exit 122, got $LASTEXITCODE" }
$env:SICO_RUNNER = 'C:\definitely\missing\sico-runner.exe'
$null = & $sico run $echo 2>$work\e6.txt
if ($LASTEXITCODE -ne 127) { throw "missing runner must exit 127, got $LASTEXITCODE" }
$env:SICO_RUNNER = Join-Path $runnerDir 'target\release\sico-runner.exe'

# Corrupt cache entries fail closed.
[IO.File]::WriteAllBytes($entry.FullName, [byte[]](1, 2, 3, 4))
$null = & $sico run $echo 2>$work\corrupt-stderr.txt
if ($LASTEXITCODE -ne 121) { throw "corrupt cache entry must exit 121, got $LASTEXITCODE" }

# JSON diagnostics contract.
& cmd.exe /c "`"$sico`" run `"$echo`" --json 2> `"$work\diag.txt`"" | Out-Null
$diagJson = ([IO.File]::ReadAllText("$work\diag.txt") | ConvertFrom-Json)
if ($diagJson.schema -cne 'sico.run.diagnostic.v0') { throw 'JSON diagnostic schema mismatch' }

Write-Output "STEP_0082_OK cache=reuse,fresh,corrupt-closed run=file,dash,exit7,reject122 eval=42,refuse120 runner-missing=127 json=sico.run.diagnostic.v0 work=$work"
