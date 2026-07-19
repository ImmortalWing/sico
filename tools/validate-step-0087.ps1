param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)

cargo test --offline --locked -p sico-ir -p sico-semantics -p sico-codegen-wasm -p sico-cli
if ($LASTEXITCODE -ne 0) { throw 'STEP-0087 workspace tests failed' }

cargo build -q --offline --locked -p sico-cli
if ($LASTEXITCODE -ne 0) { throw 'sico build failed' }
$sico = Join-Path $root 'target\debug\sico.exe'
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
$runnerDir = Join-Path $root 'runner\sico-runner'
$build = "call `"$vcvars`" && set RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc&& cd /d `"$runnerDir`" && cargo build --release --offline"
$null = & cmd.exe /c $build
if ($LASTEXITCODE -ne 0) { throw 'sico-runner build failed' }
$env:SICO_RUNNER = Join-Path $runnerDir 'target\release\sico-runner.exe'
$work = Join-Path $root 'target\evidence\step-0087'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force
$env:SICO_CACHE_DIR = Join-Path $work 'cache'

# --- structured task scope: sequential spawn/await composes exactly ---
$pair = & $sico run (Join-Path $root 'tests\end-to-end\script-task-pair.sico') 2>$work\p1e.txt
if ($LASTEXITCODE -ne 0 -or ($pair -join '') -cne 'alpha! beta!') { throw "task-pair failed: $($pair -join '')" }

# --- cancellation edge: a cancelled task terminates the scope with the typed code ---
$cancel = & $sico run (Join-Path $root 'tests\end-to-end\script-task-cancelled.sico') 2>&1
if ($LASTEXITCODE -ne 123 -or ($cancel -join '') -notmatch 'cancelled') { throw "task cancellation must be control exit 123, got $LASTEXITCODE" }

# --- structured refusals stay typed: double await (E5101), task escapes scope (E5102) ---
$doubleAwait = Join-Path $work 'double-await.sico'
(Get-Content (Join-Path $root 'tests\end-to-end\script-task-pair.sico') -Raw) -replace 'await second', 'await first' | Set-Content $doubleAwait -NoNewline
$da = & $sico run $doubleAwait 2>&1
if ($LASTEXITCODE -ne 1 -or ($da -join '') -notmatch 'E5101') { throw "double await must be E5101/exit 1, got $LASTEXITCODE $($da -join '')" }
$escape = Join-Path $work 'escape.sico'
$escapeSource = @'
async function compute() returns Int:
  return 1
end function

async function leak() returns Task[Int]:
  task group:
    let escaped = spawn compute()
    return escaped
  end task
end function
'@
[IO.File]::WriteAllText($escape, $escapeSource, [Text.UTF8Encoding]::new($false))
$es = & $sico check $escape 2>&1
if ($LASTEXITCODE -ne 1 -or ($es -join '') -notmatch 'E5102') { throw "indirect task escape must be E5102/exit 1, got $LASTEXITCODE $($es -join '')" }

# --- collect_tasks is outside the sequential v0: typed refusal, never silent ---
$collect = Join-Path $work 'collect.sico'
(Get-Content (Join-Path $root 'tests\end-to-end\script-task-pair.sico') -Raw).Replace('await collect_never(first)', 'await collect_tasks(first)') -replace 'await second', 'await collect_tasks(second)' | Set-Content $collect -NoNewline
$co = & $sico run $collect 2>&1
if (($co -join '') -notmatch 'unsupported call target collect_tasks') { throw 'collect_tasks must be a typed refusal' }

Write-Output 'STEP_0087_OK task-pair=sequential cancel-edge=123 E5101/E5102=typed-indirect collect=refused'
