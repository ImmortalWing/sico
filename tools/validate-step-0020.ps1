param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$contract = @(Get-Content -LiteralPath (Join-Path $root 'tests/cli/contract.txt') -Encoding UTF8 | Where-Object { $_.Trim() })
if ($contract.Count -ne 10) {
  throw "unexpected CLI contract row count: $($contract.Count)"
}
foreach ($command in @('check', 'format', 'outline')) {
  if (-not ($contract -match "command=.*$command")) {
    throw "CLI contract does not mention $command"
  }
}
if (-not ($contract -match 'type-checker') -or -not ($contract -match 'unavailable=run,build,-c,repl')) {
  throw 'CLI contract must expose the M1/M2 and M3/M4 capability boundary'
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-cli --all-targets --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0020 CLI integration tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0020_OK commands=3 input_modes=file,stdin exits=0,1,2 check=text,json format=stdout,check,write outline=text,json integrations=4 type_checker=unavailable run_build_repl=unavailable'
