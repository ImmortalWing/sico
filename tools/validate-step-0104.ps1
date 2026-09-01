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

# --- toolchain hygiene: fmt, clippy, focused suites ---
Invoke-NativeChecked $cargo @('fmt', '--all', '--check') 'cargo fmt check failed'
Invoke-NativeChecked $cargo @('clippy', '--workspace', '--all-targets', '--offline', '--locked', '--', '-D', 'warnings') 'clippy failed'
Invoke-NativeChecked $cargo @(
    'test', '--offline', '--locked',
    '-p', 'sico-semantics', '-p', 'sico-ir', '-p', 'sico-codegen-wasm',
    '-p', 'sico-index', '-p', 'sico-cli'
) 'STEP-0104 focused workspace tests failed'

# --- diagnostic catalog and semantic-case oracles (now 25/33) ---
& (Join-Path $root 'tools\validate-diagnostics.ps1') -RepositoryRoot $root
if (-not $?) { throw 'validate-diagnostics failed' }
& (Join-Path $root 'tools\validate-semantic-cases.ps1') -RepositoryRoot $root
if (-not $?) { throw 'validate-semantic-cases failed' }
& (Join-Path $root 'tools\validate-error-taxonomy.ps1') -RepositoryRoot $root
if (-not $?) { throw 'validate-error-taxonomy failed (catalog/taxonomy coverage drift)' }

Invoke-NativeChecked $cargo @('build', '-q', '--offline', '--locked', '-p', 'sico-cli') 'sico build failed'
$sico = Join-Path $root 'target\debug\sico.exe'
$runnerDir = Join-Path $root 'runner\sico-runner'
Invoke-NativeChecked $cargo @(
    'build', '--release', '--offline', '--locked', '--manifest-path',
    (Join-Path $runnerDir 'Cargo.toml')
) 'sico-runner build failed' | Out-Null
$env:SICO_RUNNER = Join-Path $runnerDir 'target\release\sico-runner.exe'
$work = Join-Path $root 'target\evidence\step-0104'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -Recurse -File | Remove-Item -Recurse -Force
$env:SICO_CACHE_DIR = Join-Path $work 'cache'

# --- new stable diagnostics land exactly (RFC-0036 §6) ---
$cases = @(
    @{ Path = 'syntax-candidates\b\future-task\invalid\uncollected-task.sico'; Code = 'E5103' },
    @{ Path = 'syntax-candidates\b\future-task\invalid\detached-spawn.sico'; Code = 'E5104' },
    @{ Path = 'syntax-candidates\b\future-task\invalid\scope-nesting-limit.sico'; Code = 'E5105' },
    @{ Path = 'syntax-candidates\b\affine-resources\invalid\borrow-across-await.sico'; Code = 'E5003' }
)
foreach ($case in $cases) {
    $output = & $sico check (Join-Path $root $case.Path) 2>&1
    if ($LASTEXITCODE -ne 1 -or ($output -join '') -notmatch $case.Code) {
        throw "$($case.Path) must be $($case.Code)/exit 1, got $LASTEXITCODE $($output -join '')"
    }
}

# --- faithful task IR runs the exact M9 observable behavior ---
$pair = & $sico run (Join-Path $root 'tests\end-to-end\script-task-pair.sico') 2>$work\p1e.txt
if ($LASTEXITCODE -ne 0 -or ($pair -join '') -cne 'alpha! beta!') { throw "task-pair failed: $($pair -join '')" }
$cancel = & $sico run (Join-Path $root 'tests\end-to-end\script-task-cancelled.sico') 2>&1
if ($LASTEXITCODE -ne 123 -or ($cancel -join '') -notmatch 'cancelled') { throw "task cancellation must be control exit 123, got $LASTEXITCODE" }

# --- collect_tasks executes in creation order (RFC-0036 §4/§10) ---
$collect = & $sico run (Join-Path $root 'tests\end-to-end\script-task-collect.sico') 2>$work\p2e.txt
if ($LASTEXITCODE -ne 0 -or ($collect -join '') -cne 'alpha! beta!') { throw "task-collect failed: $($collect -join '')" }

# --- historical sequential-task validator stays green under the new IR ---
& (Join-Path $root 'tools\validate-step-0087.ps1') -RepositoryRoot $root
if (-not $?) { throw 'STEP-0087 regression under RFC-0036' }

Write-Output 'STEP_0104_OK diagnostics=E5003/E5103-E5105 task-pair=exact cancel-edge=123 collect=executed step-0087=green'
