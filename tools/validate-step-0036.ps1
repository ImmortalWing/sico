param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe'),
  [string]$WasmtimePath = ''
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
if (-not $WasmtimePath) {
  $WasmtimePath = & (Join-Path $root 'tools/ensure-wasmtime.ps1') -RepositoryRoot $root
}
$WasmtimePath = (Resolve-Path $WasmtimePath).Path
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0014-minimal-build-run-cli-v0.md') -Raw -Encoding UTF8
$cliSource = Get-Content -LiteralPath (Join-Path $root 'crates/sico-cli/src/lib.rs') -Raw -Encoding UTF8
$appCliSource = Get-Content -LiteralPath (Join-Path $root 'crates/sico-app-cli/src/lib.rs') -Raw -Encoding UTF8
if ($rfc -notmatch '(?m)^> - status: accepted\r?$') { throw 'RFC-0014 is not accepted' }
foreach ($marker in @('Command::new("build")', 'compile_component(&module)')) {
  if (-not $cliSource.Contains($marker)) { throw "CLI is missing M3 contract marker: $marker" }
}
foreach ($marker in @('Command::new("pack")', 'Command::new("run")', 'run_authorized_package', 'SICO_WASMTIME')) {
  if (-not $appCliSource.Contains($marker)) { throw "application CLI is missing Runtime contract marker: $marker" }
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
$validation = Join-Path $root 'target/m3-step-0036'
New-Item -ItemType Directory -Force -Path $validation | Out-Null
$artifacts = @(
  (Join-Path $validation 'answer-1.component.wasm'),
  (Join-Path $validation 'answer-2.component.wasm'),
  (Join-Path $validation 'invalid.component.wasm')
)
foreach ($artifact in $artifacts) {
  if (Test-Path -LiteralPath $artifact -PathType Leaf) { Remove-Item -LiteralPath $artifact -Force }
}
$errorFile = Join-Path $validation 'stderr.txt'
if (Test-Path -LiteralPath $errorFile -PathType Leaf) { Remove-Item -LiteralPath $errorFile -Force }

Push-Location $root
try {
  & $CargoPath test --offline --locked -p sico-runtime -p sico-codegen-wasm -p sico-cli -p sico-app-cli --all-targets --all-features --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0036 tests failed' }
  & $CargoPath clippy --offline --locked -p sico-runtime -p sico-codegen-wasm -p sico-cli -p sico-app-cli --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0036 strict Clippy failed' }
  & $CargoPath build --offline --locked -p sico-cli -p sico-app-cli --quiet
  if ($LASTEXITCODE -ne 0) { throw 'modular Sico CLI build failed' }
  $cli = Join-Path $root 'target/debug/sico.exe'
  $appCli = Join-Path $root 'target/debug/sico-app.exe'
  $answer = Join-Path $root 'tests/end-to-end/answer.sico'

  & $cli build --output $artifacts[0] $answer | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'first deterministic build failed' }
  & $cli build --output $artifacts[1] $answer | Out-Null
  if ($LASTEXITCODE -ne 0) { throw 'second deterministic build failed' }
  $firstHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifacts[0]).Hash
  $secondHash = (Get-FileHash -Algorithm SHA256 -LiteralPath $artifacts[1]).Hash
  if ($firstHash -cne $secondHash) { throw 'CLI artifacts are not byte-identical' }

  $runtimeCases = @(
    @{ Source = 'tests/end-to-end/answer.sico'; Expected = '42' },
    @{ Source = 'tests/end-to-end/truth.sico'; Expected = 'true' },
    @{ Source = 'tests/end-to-end/unit.sico'; Expected = '()' }
  )
  foreach ($case in $runtimeCases) {
    $name = [IO.Path]::GetFileNameWithoutExtension($case.Source)
    $component = Join-Path $validation "$name.component.wasm"
    $package = Join-Path $validation "$name.sapp"
    foreach ($path in @($component, $package)) { if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path -Force } }
    & $cli build --output $component (Join-Path $root $case.Source) | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Compiler corpus failed: $($case.Source)" }
    & $appCli pack --output $package $component | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "Package corpus failed: $($case.Source)" }
    $actual = (& $appCli run --allow-unsigned-dev --runtime $WasmtimePath $package | Out-String).Trim()
    if ($LASTEXITCODE -ne 0) { throw "Runtime corpus failed: $($case.Source)" }
    if ($actual -cne $case.Expected) { throw "Runtime result mismatch for $($case.Source): $actual" }
  }

  $invalid = Join-Path $root 'syntax-candidates/b/numbers-units/invalid/text-as-int.sico'
  $ErrorActionPreference = 'Continue'
  & $cli build --output $artifacts[2] $invalid 2>$errorFile | Out-Null
  $invalidExit = $LASTEXITCODE
  $ErrorActionPreference = 'Stop'
  if ($invalidExit -ne 1) { throw 'semantic invalid source did not exit 1' }
  if (Test-Path -LiteralPath $artifacts[2]) { throw 'semantic invalid source produced an artifact' }
  if (-not (Get-Content -LiteralPath $errorFile -Raw).Contains('E2001')) { throw 'semantic invalid source lost its registered diagnostic' }

  $ErrorActionPreference = 'Continue'
  $answerPackage = Join-Path $validation 'answer.sapp'
  & $appCli run --allow-unsigned-dev --runtime (Join-Path $validation 'missing-wasmtime.exe') $answerPackage 2>$errorFile | Out-Null
  $missingRuntimeExit = $LASTEXITCODE
  $ErrorActionPreference = 'Stop'
  if ($missingRuntimeExit -ne 2) { throw 'missing Runtime did not exit 2' }

  $version = (& $WasmtimePath --version | Out-String).Trim()
  if ($version -notmatch '^wasmtime 46\.0\.1(?: |$)') { throw "unexpected Wasmtime version: $version" }
  Write-Output "STEP_0036_OK cli=sico-build,sico-app-pack-run inputs=file,stdin artifact=component-wasm,sapp deterministic=sha256 runtime=wasmtime-46.0.1 corpus=int-bool-unit semantic_invalid=exit-1-no-artifact unsupported_runtime=exit-2 overwrite=refused entry=sync-scalar-main"
}
finally {
  Pop-Location
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
