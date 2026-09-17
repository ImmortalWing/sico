param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)

$adrPath = Join-Path $root 'docs\adr\ADR-0015-compiler-bootstrap-architecture-v0.md'
$stepPath = Join-Path $root 'docs\steps\STEP-0196-m22-bootstrap-architecture-proposal.md'
foreach ($path in $adrPath, $stepPath) {
    if (-not (Test-Path -LiteralPath $path)) { throw "missing STEP-0196 artifact: $path" }
}

$adr = Get-Content -LiteralPath $adrPath -Raw -Encoding UTF8
$step = Get-Content -LiteralPath $stepPath -Raw -Encoding UTF8

$adrNeedles = @(
    '- status: accepted (owner directive "完成M22，规划M23-24", 2026-09-16)',
    'canonical `sico.ir.v0` JSON bytes',
    'There is no semantic-equivalence exception in v0.',
    'A == B == C',
    'Rust `sico-ir` verifier',
    'M7 `.sapp` producer',
    'no network, storage, credential, UI, capture or input authority',
    'five isolated runs',
    '`internal-fixture` evidence only'
)
foreach ($needle in $adrNeedles) {
    if (-not $adr.Contains($needle, [StringComparison]::Ordinal)) {
        throw "ADR-0015 invariant missing: $needle"
    }
}

foreach ($needle in @(
        'complete-contract-accepted',
        'was accepted by the owner on 2026-09-16',
        'STEP-0195''s generic-typed parameter limitation remains'
    )) {
    if (-not $step.Contains($needle, [StringComparison]::Ordinal)) {
        throw "STEP-0196 invariant missing: $needle"
    }
}

Write-Output 'STEP_0196_OK status=accepted equivalence=byte-exact generations=A-B-C verifier=rust trust=m7 evidence=internal-fixture'
