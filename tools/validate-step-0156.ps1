# Validates the M15 cross-host matrix fixture (STEP-0156):
#  1. matrix parses, schema/date current;
#  2. every case has native_hex == web_hex (byte-exact cross-host);
#  3. the harness page and guest core module exist.
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
$fixture = Join-Path $repo 'docs/evidence/m15/cross-host-matrix.json'
$json = Get-Content $fixture -Raw | ConvertFrom-Json
if ($json.schema -ne 'sico.m15.cross-host-matrix.v0') { throw 'unexpected schema id' }
if (-not $json.all_match) { throw 'cross-host matrix not fully matching' }
foreach ($case in $json.cases) {
    if (-not ($case.native_hex -and $case.web_hex -and $case.native_hex -eq $case.web_hex)) {
        throw "case $($case.name) is not byte-exact"
    }
}
foreach ($member in 'web/harness.html', 'web/guest.wordcount.core.wasm',
    'docs/evidence/m15/edge-version.txt') {
    if (-not (Test-Path (Join-Path $repo $member))) { throw "missing: $member" }
}
Write-Host 'validate-step-0156: OK (cross-host matrix, harness page, pinned engine)'

# STEP-0162: RFC-0042 UI renderer corpus must be all-pass.
$ui = Get-Content (Join-Path $repo 'docs/evidence/m15/ui-corpus.json') -Raw | ConvertFrom-Json
if ($ui.schema -ne 'sico.m15.ui-corpus.v0') { throw 'unexpected ui corpus schema' }
if (-not $ui.all_pass) { throw 'ui corpus has failing cases' }
if (-not (Test-Path (Join-Path $repo 'web/ui-corpus.html'))) { throw 'ui corpus page missing' }
Write-Host 'validate-step-0156: UI renderer corpus OK (8/8)'
