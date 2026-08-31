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
    'test', '--offline', '--locked', '-p', 'sico-ir', '-p', 'sico-semantics',
    '-p', 'sico-codegen-wasm', '-p', 'sico-cli'
) 'STEP-0087 workspace tests failed'

Invoke-NativeChecked $cargo @('build', '-q', '--offline', '--locked', '-p', 'sico-cli') 'sico build failed'
$sico = Join-Path $root 'target\debug\sico.exe'
$runnerDir = Join-Path $root 'runner\sico-runner'
Invoke-NativeChecked $cargo @(
    'build', '--release', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml')
) 'sico-runner build failed'
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

# --- collect_tasks executes under the sequential-v1 projection (RFC-0036 §4/§10) ---
$collect = & $sico run (Join-Path $root 'tests\end-to-end\script-task-collect.sico') 2>$work\p4e.txt
if ($LASTEXITCODE -ne 0 -or ($collect -join '') -cne 'alpha! beta!') { throw "task-collect failed: $($collect -join '')" }

Write-Output 'STEP_0087_OK task-pair=sequential cancel-edge=123 E5101/E5102=typed-indirect collect=executed'
