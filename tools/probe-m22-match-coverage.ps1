<#
.SYNOPSIS
M22 S5 `match` site coverage probe (STEP-0536, owner audit R0.4).

.DESCRIPTION
The `match` construct is the largest uncovered shape in the frozen source set S
(519 statements).  STEP-0534 measured the family, STEP-0535 proved four
**synthetic** fixtures byte for byte at the component layer -- and then wrote
"42 / 519 sites", an extrapolation the owner audit (AGENTS.md §10.1, §10.4
R0.4) rejects.

This probe replaces the extrapolation with a **frozen site ledger**:

  1. `tools/census-m22-match-sites.py --check` proves the ledger still
     describes the frozen S -- the same 396 candidate sites, at the same lines,
     with the same heads and the same parameter counts.
  2. The firmware test `sico_compiler_reproduces_the_frozen_match_site_ledger`
     re-derives every site from the frozen sources, compares the guest against
     the Rust oracle on the canonical IR axis **and** the Component axis (with a
     real `wasmparser::Validator` run), and checks the recorded verdict.
  3. The verdict distribution is printed so a reader sees how much of the
     family is really covered.

STEP-0536 (M-g1) removed the checked-arithmetic route's parameter-count gate,
so the candidate set grew from the 42 sites the gate let through to all 396
`U64.checked_add` match statements of the frozen S -- the 42 plus the 354 the
gate used to refuse.

Exit codes: 0 = every site keeps its recorded verdict,
1 = the ledger drifted, a site moved, or a site has no verdict yet.

Usage:
  powershell -ExecutionPolicy Bypass -File tools/probe-m22-match-coverage.ps1
  powershell -ExecutionPolicy Bypass -File tools/probe-m22-match-coverage.ps1 -Json
#>
[CmdletBinding()]
param(
    [switch]$Json
)

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$PSNativeCommandUseErrorActionPreference = $false

$repoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $repoRoot
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { Write-Error 'cargo not found'; exit 1 }
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
$env:SICO_SELFHOST_COMPONENT_CACHE = '0'
$env:RUST_TEST_THREADS = '1'
$env:SICO_PYTHON = & (Join-Path $repoRoot 'tools\ensure-python.ps1')
$pythonDir = Split-Path -Parent $env:SICO_PYTHON
$env:Path = "$pythonDir;$env:Path"

$ledgerPath = Join-Path $repoRoot 'selfhost\match-component-coverage-v1.json'
if (-not (Test-Path $ledgerPath)) {
    Write-Error "the match site ledger is missing: $ledgerPath"
    exit 1
}

Write-Host '=== match site ledger: census check' -ForegroundColor Cyan
& $env:SICO_PYTHON tools\census-m22-match-sites.py --check
if ($LASTEXITCODE -ne 0) {
    Write-Host 'MATCH_COVERAGE_MOVED: the ledger no longer describes the frozen S' -ForegroundColor Red
    exit 1
}

Write-Host '=== match site ledger: per-site differential' -ForegroundColor Cyan
& $cargo test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_compiler -- --test-threads=1 sico_compiler_reproduces_the_frozen_match_site_ledger
if ($LASTEXITCODE -ne 0) {
    Write-Host 'MATCH_COVERAGE_MOVED: a site verdict moved' -ForegroundColor Red
    exit 1
}

$ledger = Get-Content $ledgerPath -Raw | ConvertFrom-Json
$groups = $ledger.sites | Group-Object verdict | Sort-Object Name
$summary = foreach ($group in $groups) {
    [pscustomobject]@{
        Verdict = $group.Name
        Sites = $group.Count
    }
}
$pending = @($ledger.sites | Where-Object { $_.verdict -eq 'pending' }).Count
$equal = @($ledger.sites | Where-Object { $_.verdict -eq 'ir-equal+component-equal' }).Count

if ($Json) {
    [pscustomobject]@{
        Schema = $ledger.schema
        Sites = $ledger.sites.Count
        Pending = $pending
        ComponentEqual = $equal
        Verdicts = @($summary)
    } | ConvertTo-Json -Depth 5
} else {
    foreach ($row in $summary) {
        Write-Output ("{0,-52} {1,4}" -f $row.Verdict, $row.Sites)
    }
    Write-Output ("sites={0} pending={1} component-equal={2}" -f $ledger.sites.Count, $pending, $equal)
}

if ($pending -gt 0) {
    Write-Host ("MATCH_COVERAGE_PENDING {0}" -f $pending) -ForegroundColor Red
    exit 1
}
Write-Host 'MATCH_COVERAGE_OK' -ForegroundColor Green
exit 0
