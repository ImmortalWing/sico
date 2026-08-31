param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$prelude = Get-Content -LiteralPath (Join-Path $root 'semantics/prelude-v0.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$snapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/hir/b-shapes.txt') -Encoding UTF8 | Where-Object { $_.Trim() })
$bCases = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-candidates/b') -Recurse -File -Filter '*.sico')
$mutations = @(Get-ChildItem -LiteralPath (Join-Path $root 'syntax-mutations/b') -File -Filter '*.sico')

if ($prelude.schema -ne 'sico.m2.prelude.v0' -or @($prelude.types).Count -ne 16 -or @($prelude.p0_fixture_members).Count -ne 3) {
  throw 'unexpected M2 prelude contract shape'
}
$typeNames = @($prelude.types | ForEach-Object name | Sort-Object -Unique)
if ($typeNames.Count -ne 16 -or -not ($typeNames -contains 'Int') -or -not ($typeNames -contains 'Result')) {
  throw 'prelude type names are incomplete or duplicated'
}
if ($prelude.namespaces.cross_namespace_collision -notmatch 'requires a new case') {
  throw 'undecided namespace collision must remain an explicit RFC gate'
}
if ($bCases.Count -ne 58 -or $snapshots.Count -ne 58 -or $mutations.Count -ne 12) {
  throw "HIR evidence mismatch: cases=$($bCases.Count) snapshots=$($snapshots.Count) mutations=$($mutations.Count)"
}

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
try {
  & $CargoPath test --offline --locked -p sico-hir --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0022 HIR tests failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0022_OK b_lowered=58 snapshots=58 stable_ids=pass source_maps=pass semantic_tokens=complete mutations_blocked=12 prelude_types=16 fixture_members=3 namespace_collision=undecided-rfc-gated'
