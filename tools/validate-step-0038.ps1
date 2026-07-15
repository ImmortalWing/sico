param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0015-sapp-package-format-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0038-sapp-threat-model-package-contract.md') -Raw -Encoding UTF8
$matrix = Get-Content -LiteralPath (Join-Path $root 'tests/packages/threat-matrix.json') -Raw -Encoding UTF8 | ConvertFrom-Json

if ($rfc -notmatch '(?m)^> - status: accepted\r?$') { throw 'RFC-0015 is not accepted' }
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0038 is not complete' }
if ($matrix.schema -cne 'sico.sapp.threat-matrix.v0' -or $matrix.case_count -ne 18 -or @($matrix.cases).Count -ne 18) {
  throw 'package threat matrix count/schema drifted'
}
$ids = @($matrix.cases | ForEach-Object id | Sort-Object -Unique)
if ($ids.Count -ne 18 -or $ids[0] -cne 'PKG-001' -or $ids[-1] -cne 'PKG-018') { throw 'package threat IDs are not unique/stable' }
foreach ($needle in @('fixed framing + canonical JSON', 'SHA-256', 'Ed25519', '64 MiB', '32 MiB', '8 MiB')) {
  if (-not $rfc.Contains($needle)) { throw "RFC-0015 missing contract: $needle" }
}
Write-Output 'STEP_0038_OK format=sapp-v0 archive=fixed-uncompressed manifest=canonical-json threat_cases=18 limits=6 hash=sha256 signature_domain=ed25519-dev-v0 next=STEP-0039'
