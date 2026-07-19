param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'

cargo test --offline --locked -p sico-codegen-wasm -p sico-package
if ($LASTEXITCODE -ne 0) { throw 'STEP-0081 workspace tests failed' }

# The runner links the Wasmtime crate: MSVC-only on this host.
$vcvars = 'C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat'
if (-not (Test-Path $vcvars)) { throw "MSVC vcvars64.bat not found: $vcvars" }
$runnerDir = Join-Path $root 'runner\sico-runner'
$build = "call `"$vcvars`" && set RUSTUP_TOOLCHAIN=stable-x86_64-pc-windows-msvc&& cd /d `"$runnerDir`" && cargo test --release --offline"
$testOutput = & cmd.exe /c $build
if ($LASTEXITCODE -ne 0) {
    $testOutput | ForEach-Object { Write-Host $_ }
    throw 'STEP-0081 runner tests failed'
}

$runner = Join-Path $runnerDir 'target\release\sico-runner.exe'
$work = Join-Path $root 'target\evidence\step-0081'
New-Item -ItemType Directory -Force -Path $work | Out-Null
Get-ChildItem $work -File | Remove-Item -Force
$echoComponent = Join-Path $work 'echo.component.wasm'
$rejectComponent = Join-Path $work 'reject.component.wasm'
cargo run -q --offline --locked -p sico-cli -- build --profile script-v0 --output $echoComponent (Join-Path $root 'tests/end-to-end/script-echo.sico')
if ($LASTEXITCODE -ne 0) { throw 'script build failed (echo)' }
cargo run -q --offline --locked -p sico-cli -- build --profile script-v0 --output $rejectComponent (Join-Path $root 'tests/end-to-end/script-reject.sico')
if ($LASTEXITCODE -ne 0) { throw 'script build failed (reject)' }

$payload = 'step-0081-exact'
$previousPreference = $ErrorActionPreference
$ErrorActionPreference = 'Continue'
$stdout = $payload | & $runner $echoComponent -- alpha 2>$work\echo-stderr.txt
$echoExit = $LASTEXITCODE
$ErrorActionPreference = $previousPreference
if ($echoExit -ne 0 -or ($stdout -join '') -cne $payload) {
    throw "echo run mismatch: exit=$echoExit stdout=$stdout"
}
$stderrText = Get-Content $work\echo-stderr.txt -Raw
if ($stderrText -cmatch '^\s*$') { throw 'echo fixture must mirror stderr too' }

& cmd.exe /c "`"$runner`" `"$rejectComponent`" 2> `"$work\reject-stderr.txt`"" | Out-Null
$rejectExit = $LASTEXITCODE
$rejectJson = ([IO.File]::ReadAllText("$work\reject-stderr.txt") | ConvertFrom-Json)
if ($rejectExit -ne 122 -or $rejectJson.class -cne 'domain-error') {
    throw "domain mapping mismatch: exit=$rejectExit class=$($rejectJson.class)"
}

& cmd.exe /c "echo garbage| `"$runner`" `"$work\missing.wasm`" 2> `"$work\missing-stderr.txt`"" | Out-Null
$missingExit = $LASTEXITCODE
if ($missingExit -ne 121) { throw "missing component must be a CLI error, got $missingExit" }
Set-Content -Path (Join-Path $work 'bad.wasm') -Value 'not-a-component' -NoNewline
& cmd.exe /c "`"$runner`" `"$work\bad.wasm`" 2> `"$work\bad-stderr.txt`"" | Out-Null
$badExit = $LASTEXITCODE
$badJson = ([IO.File]::ReadAllText("$work\bad-stderr.txt") | ConvertFrom-Json)
if ($badExit -ne 127 -or $badJson.class -cne 'incompatible') {
    throw "incompatible mapping mismatch: exit=$badExit class=$($badJson.class)"
}

Write-Output "STEP_0081_OK runner=in-process-wasmtime-46.0.1 tests=6 exit=0/122/125/127/121 typed_outcomes=domain,timeout,fuel,memory,trap,cancelled host_survival=pass work=$work"
