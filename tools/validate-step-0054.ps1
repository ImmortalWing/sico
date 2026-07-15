param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$matrix = Get-Content -LiteralPath (Join-Path $root 'tests/android-host/threat-matrix.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$environment = Get-Content -LiteralPath (Join-Path $root 'tests/android-host/environment-2026-07-16.json') -Raw -Encoding UTF8 | ConvertFrom-Json
$adr = Get-Content -LiteralPath (Join-Path $root 'docs/adr/ADR-0005-android-host-runtime-lifecycle-boundary-v0.md') -Raw -Encoding UTF8
$rfc = Get-Content -LiteralPath (Join-Path $root 'docs/rfc/RFC-0021-android-intent-jni-contract-v0.md') -Raw -Encoding UTF8
$step = Get-Content -LiteralPath (Join-Path $root 'docs/steps/STEP-0054-android-host-contract.md') -Raw -Encoding UTF8
if ($matrix.schema -ne 'sico.android.threat-matrix.v0' -or @($matrix.cases).Count -ne 30 -or @($matrix.cases.id | Sort-Object -Unique).Count -ne 30) { throw 'Android threat matrix is incomplete' }
foreach ($area in 'intent','uri','identity','permission','lifecycle','runtime','ui','packaging') { if ($area -notin $matrix.cases.area) { throw "Android threat area missing: $area" } }
if ($environment.local_evidence_ceiling -ne 'cross-compile-check-only' -or $environment.device_or_avd -ne 'unavailable') { throw 'Android environment evidence is overstated' }
if ($adr -notmatch '(?m)^> - status: accepted\r?$' -or -not $adr.Contains('digest') -or -not $adr.Contains('Pulley')) { throw 'ADR-0005 is incomplete' }
if ($rfc -notmatch '(?m)^> - status: accepted\r?$' -or -not $rfc.Contains('sico.android.bridge.v0')) { throw 'RFC-0021 is incomplete' }
if ($step -notmatch '(?m)^> - status: complete\r?$') { throw 'STEP-0054 is not complete' }
$targets = & rustup target list --installed
foreach ($target in 'aarch64-linux-android','x86_64-linux-android') { if ($target -notin $targets) { throw "Rust Android target missing: $target" } }
Write-Output 'STEP_0054_OK threats=30 rust_targets=aarch64,x86_64 sdk=unavailable ndk=unavailable runner=unavailable bridge=typed-json uri=copy-then-verify lifecycle=descriptor-restore evidence_ceiling=cross-compile-check next=STEP-0055'
