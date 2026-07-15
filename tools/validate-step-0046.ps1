param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$matrix = Get-Content -LiteralPath (Join-Path $root 'tests/desktop-host/threat-matrix.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$adr = Get-Content -LiteralPath (Join-Path $root 'docs/adr/ADR-0004-desktop-host-identity-lifecycle-platform-v0.md') -Raw -Encoding UTF8
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0020-desktop-ui-permission-contract-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0046-desktop-host-threat-lifecycle-contract.md') -Raw -Encoding UTF8
if ($matrix.schema -ne 'sico.desktop-host.threat-matrix.v0' -or @($matrix.cases).Count -ne 24) { throw 'Desktop Host threat matrix is incomplete' }
$classes = @($matrix.cases | ForEach-Object class | Sort-Object -Unique)
if ($classes.Count -ne 6) { throw 'Desktop Host threat class coverage drifted' }
if ($adr -notmatch '(?m)^> - status: accepted\r?$' -or $rfc -notmatch '(?m)^> - status: accepted\r?$' -or $step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0046 decisions are not accepted/complete' }
foreach ($needle in @('AppIdentityKey', 'RevisionDigest', 'CapabilityFingerprint', 'one supervised guest', 'runtime-verified')) {
  if (-not $adr.Contains($needle)) { throw "Desktop Host ADR missing: $needle" }
}
Write-Output 'STEP_0046_OK threats=24 identity=app-plus-trust revision=package-digest permission=capability-fingerprint install=signed-only lifecycle=single-supervised ui=typed-bounded platform_evidence=explicit next=STEP-0047'
