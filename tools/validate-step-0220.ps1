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
if ($entries.Count -ne 215 -or $accepted.Count -ne 37 -or $refused.Count -ne 178) {
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
if ($classes['LEXICAL'] -ne 116 -or $classes['SCRIPT_ABI'] -ne 28) {
    throw 'Script refusal class counts drifted'
}

Write-Output 'STEP_0220_OK corpus=215 script_accept=37 reproducible=37 script_refuse=178 lexical=116 script_abi=28 semantic=34 artifact_leaks=0'
