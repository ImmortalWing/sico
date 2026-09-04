﻿param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
. (Join-Path $root 'tools\lib\native-command.ps1')

# --- oracle reruns backing criterion (a)/(c) ---
& (Join-Path $root 'tools\validate-semantic-query.ps1') -RepositoryRoot $root
if (-not $?) { throw 'semantic-query oracle failed' }
& (Join-Path $root 'tools\validate-error-taxonomy.ps1') -RepositoryRoot $root
if (-not $?) { throw 'taxonomy oracle failed' }
& (Join-Path $root 'tools\validate-ai-eval.ps1') -RepositoryRoot $root
if (-not $?) { throw 'ai-eval dataset oracle failed' }
& (Join-Path $root 'tools\validate-diagnostics.ps1') -RepositoryRoot $root
if (-not $?) { throw 'diagnostics oracle failed' }
& (Join-Path $root 'tools\validate-semantic-cases.ps1') -RepositoryRoot $root
if (-not $?) { throw 'semantic-case oracle failed' }

# --- benchmarks reproducible ---
Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-index', '--test', 'semantic_bench', '--', '--nocapture') 'semantic-bench-failed|STEP-0123'
Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-mcp-server') 'mcp-transport-tests-failed|STEP-0123'
Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-ai-tools') 'ai-tool-tests-failed|STEP-0123'

# --- audit document consistency: GO where provable, blocked where not ---
$audit = Get-Content (Join-Path $root 'docs\steps\STEP-0123-ai-tooling-closure-audit.md') -Raw -Encoding UTF8
foreach ($needle in @(
        '(a) Query protocol stability', '(b) Structured stable diagnostics',
        '(c) Reproducible benchmarks', '(d) Numeric quality budgets met',
        'M13: GO (3/4 criteria GO'
    )) {
    if ($audit -notlike "*$needle*") { throw "audit claim missing: $needle" }
}
# A target must never be claimed MET from subagent runs (the doc may
# only say they cannot meet them).
if ($audit -match 'target budgets[^.]*?met by[^.]*?subagent') { throw 'target wrongly claimed from subagent evidence' }
if ($audit -match 'targets? (?:are|is) met') { throw 'target wrongly claimed met' }

# --- ADR-0011 tiers still consistent with the baseline ---
$adr = Get-Content (Join-Path $root 'docs\adr\ADR-0011-ai-quality-budgets.md') -Raw -Encoding UTF8
if ($adr -notlike '*0.974359*') { throw 'ADR-0011 baseline reference missing' }
if ($adr -notlike '*blocked-external-evidence*') { throw 'ADR-0011 target deferral missing' }

Write-Output 'STEP_0123_OK criteria=a:GO,b:GO,c:GO,d:blocked-external-evidence verdict=M13-GO oracles=5-green benches=reproducible'
