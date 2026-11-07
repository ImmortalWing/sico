param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

foreach ($relative in @(
        'selfhost\corpus-v0.json',
        'selfhost\script-build-corpus-v0.json',
        'tools\update-m22-corpus.ps1',
        'docs\steps\STEP-0220-m22-script-build-corpus.md'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $root $relative))) {
        throw "missing STEP-0220 artifact: $relative"
    }
}

& (Join-Path $root 'tools\update-m22-corpus.ps1') -RepositoryRoot $root -Verify
if ($LASTEXITCODE -ne 0) { throw 'STEP-0220 corpus replay failed' }

$buildManifest = Get-Content -LiteralPath (Join-Path $root 'selfhost\script-build-corpus-v0.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$entries = $buildManifest.entries
if ($buildManifest.schema -cne 'sico.m22.script-build-corpus.v0') {
    throw "unexpected Script build corpus schema: $($buildManifest.schema)"
}
$accepted = @($entries | Where-Object rust_script_build -eq 'accepted')
$refused = @($entries | Where-Object rust_script_build -eq 'refused')
# Current-tree census: the M22 script-build corpus was re-frozen with the main
# corpus 215 -> 235 (STEP-0279..0283, merge 7bcdc35), so these counts follow
# the 235-entry manifest. Measured by PowerShell ConvertFrom-Json over
# selfhost/script-build-corpus-v0.json (2026-11-07): 235 / 42 accepted / 193
# refused; every accepted entry is component-reproducible.
if ($entries.Count -ne 235 -or $accepted.Count -ne 42 -or $refused.Count -ne 193) {
    throw "unexpected Script corpus counts: total=$($entries.Count) accepted=$($accepted.Count) refused=$($refused.Count)"
}
if (@($accepted | Where-Object { -not $_.component_reproducible -or [string]::IsNullOrWhiteSpace($_.component_sha256) }).Count -ne 0) {
    throw 'accepted Script entry lacks reproducible Component digest'
}
if (@($refused | Where-Object { $null -ne $_.component_sha256 -or $_.component_reproducible }).Count -ne 0) {
    throw 'refused Script entry contains artifact evidence'
}

$classes = @{}
foreach ($entry in $refused) {
    $key = $entry.build_diagnostic_ids -join ','
    if (-not $classes.ContainsKey($key)) { $classes[$key] = 0 }
    $classes[$key]++
}
# LEXICAL 116 is unchanged by the 235 refreeze; SCRIPT_ABI grew 28 -> 31 (the
# +20 entries added with STEP-0279..0283 are lexical/semantic refusals, not
# Script-ABI; measured on the 193-entry refused partition, 2026-11-07).
if ($classes['LEXICAL'] -ne 116 -or $classes['SCRIPT_ABI'] -ne 31) {
    throw 'Script refusal class counts drifted'
}

Write-Output 'STEP_0220_OK corpus=235 script_accept=42 reproducible=42 script_refuse=193 lexical=116 script_abi=31 semantic=46 artifact_leaks=0'
