param(
    [string]$CargoPath = "$env:USERPROFILE\.cargo\bin\cargo.exe"
)

$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$planPath = Join-Path $root 'docs\plans\M22-M26-completion-plan-v1.md'
if (-not (Test-Path -LiteralPath $planPath)) {
    throw 'STEP-0323 completion plan missing'
}
$plan = Get-Content -LiteralPath $planPath -Raw -Encoding UTF8
foreach ($marker in @(
    'owner directive 2026-09-30',
    'GitHub',
    'user-call RHS'
)) {
    if (-not $plan.Contains($marker)) {
        throw "missing STEP-0323 plan marker: $marker"
    }
}
$accepted = 'accepted by owner directive 2026-09-30'
foreach ($doc in @(
    'docs\rfc\RFC-0048-language-v1-batch3-infix-comparison-logical-and.md',
    'docs\rfc\RFC-0049-language-v1-batch3-let-bound-if-match.md',
    'docs\rfc\RFC-0050-image-codec-jpeg-png-v0.md',
    'docs\rfc\RFC-0051-model-provenance-manifest-v0.md',
    'docs\rfc\RFC-0052-native-ui-source-binding-v0.md',
    'docs\rfc\RFC-0053-pdf-structural-reading-v0.md',
    'docs\rfc\RFC-0054-pdf-limits-refusals-writer-v0.md',
    'docs\adr\ADR-0018-native-renderer-fluent-subset-v0.md'
)) {
    $text = Get-Content -LiteralPath (Join-Path $root $doc) -Raw -Encoding UTF8
    if (-not $text.Contains($accepted)) {
        throw "contract not marked accepted: $doc"
    }
}
& (Join-Path $PSScriptRoot 'validate-step-0322.ps1') -CargoPath $CargoPath
if ($LASTEXITCODE -ne 0) { throw 'STEP-0322 inherited validator failed' }
Write-Output 'STEP_0323_OK completion-plan=v1 contracts=8-accepted-by-directive gates=unchanged external-deferrals=registered'
