param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$manifest = Get-Content -LiteralPath (Join-Path $root 'syntax-mutations/manifest.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$map = Get-Content -LiteralPath (Join-Path $root 'diagnostics/syntax-mutation-map.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$snapshot = @(Get-Content -LiteralPath (Join-Path $root 'tests/diagnostics/b-mutations.snap') -Encoding UTF8 | Where-Object { $_.Trim() })
$bMutations = @($manifest.entries | Where-Object { $_.syntax -eq 'B' })

if ($bMutations.Count -ne 12 -or @($map.cases).Count -ne 12 -or $snapshot.Count -ne 12) {
  throw "STEP-0018 corpus mismatch: mutations=$($bMutations.Count) map=$(@($map.cases).Count) snapshots=$($snapshot.Count)"
}

$expectedMutations = @($bMutations | ForEach-Object { $_.mutation } | Sort-Object -Unique)
$mappedMutations = @($map.cases | ForEach-Object { $_.mutation } | Sort-Object -Unique)
if (@(Compare-Object $expectedMutations $mappedMutations).Count -ne 0) {
  throw 'syntax mutation map does not cover exactly the B mutation identities'
}

$snapshotMutations = @($snapshot | ForEach-Object { ($_ -split '\|', 2)[0] } | Sort-Object -Unique)
if (@(Compare-Object $expectedMutations $snapshotMutations).Count -ne 0) {
  throw 'diagnostic snapshot does not cover exactly the B mutation identities'
}

$codes = @($map.cases | ForEach-Object { $_.code } | Sort-Object -Unique)
$anchors = @($map.cases | ForEach-Object { $_.anchor } | Sort-Object -Unique)
if ($codes.Count -ne 12 -or $anchors.Count -ne 5) {
  throw "unexpected syntax diagnostic identity/anchor count: codes=$($codes.Count) anchors=$($anchors.Count)"
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-parser -p sico-diagnostics --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0018 Rust tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

& (Join-Path $root 'tools/validate-diagnostics.ps1') -RepositoryRoot $root
if ($LASTEXITCODE -ne 0) { throw 'diagnostic catalog validation failed' }

Write-Output "STEP_0018_OK mutations=12 root_causes=12 diagnostics=12 anchors=$($anchors.Count) snapshots=$($snapshot.Count) cascade_outside_construct_max=0 text_json_spans=pass lossless_recovery=pass semantic_ast_on_error=blocked"
