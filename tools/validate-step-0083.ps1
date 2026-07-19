param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)

cargo test --offline --locked -p sico-ir -p sico-semantics -p sico-codegen-wasm -p sico-cli -p sico-package
if ($LASTEXITCODE -ne 0) { throw 'STEP-0083 workspace tests failed' }

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
$work = Join-Path $root 'target\evidence\step-0083'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force
$env:SICO_CACHE_DIR = Join-Path $work 'cache'

# --- Pilot 1: args echo (text.join over List[Text], UTF-8 passthrough) ---
$echo = & $sico run (Join-Path $root 'tests\end-to-end\script-args-echo.sico') -- alpha beta 雪 2>$work\p1e.txt
if ($LASTEXITCODE -ne 0 -or ($echo -join '') -cne 'alpha beta 雪') { throw "args-echo pilot failed: $($echo -join '')" }

# --- Pilot 2: word count (split_words/trim over stdin, UTF-8 guard) ---
$wc = 'hello world  from sico' + "`n" + 'second line here' | & $sico run (Join-Path $root 'tests\end-to-end\script-word-count.sico') 2>$work\p2e.txt
if ($LASTEXITCODE -ne 0 -or ($wc -join '') -cne '7') { throw "word-count pilot failed: $($wc -join '')" }
$wcBad = [IO.File]::ReadAllBytes((Join-Path $root 'tests\end-to-end\script-word-count.sico')) | Out-Null
$notUtf8 = [byte[]](0xFF, 0xFE, 0x41)
& cmd.exe /c "echo `"$([char]0xFF)$([char]0xFE)`" | `"$sico`" run `"$root\tests\end-to-end\script-word-count.sico`" 2>NUL" | Out-Null
# Direct invalid-UTF-8 stdin check through a temp file to avoid shell encoding.
$bin = Join-Path $work 'bad-stdin.bin'
[IO.File]::WriteAllBytes($bin, $notUtf8)
$wcOut = & cmd.exe /c "`"$sico`" run `"$root\tests\end-to-end\script-word-count.sico`" < `"$bin`" 2>&1"
if ($LASTEXITCODE -ne 122 -or ($wcOut -join '') -notmatch 'invalid-input') { throw "word-count invalid UTF-8 must fail closed with 122, got $LASTEXITCODE $($wcOut -join '')" }

# --- Pilot 3: JSON filter (bounded parser, helper function aggregate ABI) ---
$json = Join-Path $root 'tests\end-to-end\script-json-filter.sico'
$jf1 = '{"name":"sico","n":42}' | & $sico run $json 2>$work\p3e1.txt
if ($LASTEXITCODE -ne 0 -or ($jf1 -join '') -cne '"sico"') { throw "json-filter hit failed: $($jf1 -join '')" }
$jf2 = '{"other":1}' | & $sico run $json 2>&1
if ($LASTEXITCODE -ne 122 -or ($jf2 -join '') -notmatch 'missing name field') { throw 'json-filter missing-key refusal failed' }
$jf3 = '{bad' | & $sico run $json 2>&1
if ($LASTEXITCODE -ne 122 -or ($jf3 -join '') -notmatch 'malformed JSON') { throw 'json-filter malformed refusal failed' }

# --- Pilot 4: scoped file transform (sico.fs through Host-granted roots) ---
$transform = Join-Path $root 'tests\end-to-end\script-file-transform.sico'
$escape = Join-Path $root 'tests\end-to-end\script-fs-escape.sico'
$readRoot = Join-Path $work 'read'
$writeRoot = Join-Path $work 'write'
New-Item -ItemType Directory -Force -Path $readRoot, $writeRoot | Out-Null
'hello scoped files from sico' | Set-Content (Join-Path $readRoot 'input.txt') -NoNewline
$fs1 = & $sico run --fs-read-root $readRoot --fs-write-root $writeRoot $transform 2>$work\p4e1.txt
if ($LASTEXITCODE -ne 0 -or ($fs1 -join '') -cne 'transformed') { throw "file-transform pilot failed: $($fs1 -join '')" }
$written = Get-Content (Join-Path $writeRoot 'output.txt') -Raw
if ($written -cne 'words: 5') { throw "file-transform wrote unexpected content: $written" }

# Denied by default: no grants at all.
$fs2 = & $sico run $transform 2>&1
if ($LASTEXITCODE -ne 122 -or ($fs2 -join '') -notmatch 'storage\.read not granted') { throw 'fs without grants must fail closed with 122' }
# Read granted, write not granted.
$fs3 = & $sico run --fs-read-root $readRoot $transform 2>&1
if ($LASTEXITCODE -ne 122 -or ($fs3 -join '') -notmatch 'storage\.write not granted') { throw 'fs write without grant must fail closed with 122' }
# Missing file below a granted root.
Remove-Item (Join-Path $readRoot 'input.txt')
$fs4 = & $sico run --fs-read-root $readRoot --fs-write-root $writeRoot $transform 2>&1
if ($LASTEXITCODE -ne 122 -or ($fs4 -join '') -notmatch 'not found below any granted read root') { throw 'fs not-found must be a typed 122' }
'hello scoped files from sico' | Set-Content (Join-Path $readRoot 'input.txt') -NoNewline
# Path escape above the granted read root.
$fs5 = & $sico run --fs-read-root $readRoot --fs-write-root $writeRoot $escape 2>&1
if ($LASTEXITCODE -ne 122 -or ($fs5 -join '') -notmatch 'path escapes the granted scope') { throw 'fs escape must fail closed with 122' }

Write-Host 'STEP-0083 validation: all four pilots and every fs refusal passed'
