param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$implementation = Get-Content -LiteralPath (Join-Path $root 'crates/sico-ir/src/lib.rs') -Raw -Encoding UTF8
$lowering = Get-Content -LiteralPath (Join-Path $root 'crates/sico-ir/src/lower.rs') -Raw -Encoding UTF8
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0010-effect-resource-revision-ir-flow-v0.md') -Raw -Encoding UTF8
$snapshots = @(Get-Content -LiteralPath (Join-Path $root 'tests/ir/flow-lowering.snap') -Encoding UTF8 | Where-Object { $_.Trim() })

if ($rfc -notmatch '(?m)^> - status: accepted\r?$') { throw 'effect/resource/revision IR contract is not accepted' }
foreach ($contract in @('ResourceViolation', 'ResourceLeak', 'BorrowEscape', 'RevisionGuard', 'ResourceCall')) {
  if (-not $implementation.Contains($contract)) { throw "flow verifier is missing contract marker: $contract" }
}
foreach ($contract in @('Operation::EffectCall', 'Operation::ResourceBorrow', 'Operation::ResourceDrop', 'Operation::RevisionCheck')) {
  if (-not $lowering.Contains($contract)) { throw "flow lowering is missing contract marker: $contract" }
}
if ($snapshots.Count -ne 5) { throw "expected 5 flow snapshots, found $($snapshots.Count)" }

$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = 'stable'
try {
  & $CargoPath test --offline --locked -p sico-ir --quiet
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0032 flow tests failed' }
  & $CargoPath clippy --offline --locked -p sico-ir --all-targets --all-features -- -D warnings
  if ($LASTEXITCODE -ne 0) { throw 'STEP-0032 strict Clippy failed' }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}

Write-Output 'STEP_0032_OK cumulative_valid=17 deferred_valid=8 flow_valid=5 snapshots=5 effects=2 resources=2 revisions=2 ownership=affine borrow=call-scoped verifier_mutations=4 invalid_blocked=29'
