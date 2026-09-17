param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$report = Get-Content -LiteralPath (Join-Path $root 'docs\reports\m22-interim-audit.md') -Raw -Encoding UTF8

foreach ($needle in @(
        'NO-GO / implementation in progress',
        'L1 formatter byte-exact + idempotent on frozen corpora',
        'L1 checker declared diagnostic differential',
        'Lexer/token/AST parser',
        'Canonical typed IR + Rust verifier differential',
        'Sico-native deterministic codegen',
        'Frozen corpus artifact equality',
        'A=B=C self-compilation',
        'M7 `.sapp` verify/execute',
        'wall/fuel/memory/output measurement',
        'M0-M21 regression + exit audit'
    )) {
    if (-not $report.Contains($needle, [StringComparison]::Ordinal)) {
        throw "M22 interim audit invariant missing: $needle"
    }
}

Write-Output 'STEP_0200_OK verdict=NO-GO gates=10 claim=probe-not-slice-completion'
