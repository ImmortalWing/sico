param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path

function Read-RepoFile([string]$relativePath) {
    $path = Join-Path $root $relativePath
    if (-not (Test-Path -LiteralPath $path -PathType Leaf)) { throw "missing file: $relativePath" }
    Get-Content -LiteralPath $path -Raw -Encoding UTF8
}

$handbook = Read-RepoFile 'docs/platforms/LINUX-DEVELOPMENT.md'
$index = Read-RepoFile 'docs/platforms/README.md'
$step = Read-RepoFile 'docs/steps/STEP-0071-linux-development-handbook.md'
$hostSource = Read-RepoFile 'crates/sico-desktop-host/src/lib.rs'
$platform = Read-RepoFile 'crates/sico-desktop-host/src/platform.rs'
$contract = Read-RepoFile 'tests/platform/linux-documentation-contract.json' | ConvertFrom-Json

if ($contract.schema -ne 'sico.linux-documentation-contract.v0' -or $contract.claim -ne 'documentation-only' -or $contract.status -ne 'contract-verified-not-runtime-verified') {
    throw 'Linux documentation contract schema, claim or status is invalid'
}
if (-not $contract.existing.desktopHostCrate -or -not $contract.existing.associationArtifacts -or $contract.existing.associationApply -or $contract.existing.nativePermissionDialog -or $contract.existing.nativeUiRenderer -or $contract.existing.linuxRunnerEvidence -or $contract.existing.fileOpenIsCurrentlyRunnable) {
    throw 'Linux current implementation evidence is overstated'
}
if ($contract.targets.primary -ne 'x86_64-unknown-linux-gnu' -or $contract.targets.secondaryProposed -ne 'aarch64-unknown-linux-gnu' -or $contract.targets.libc -ne 'glibc' -or -not $contract.targets.minimumGlibcMustBeFrozen) {
    throw 'Linux proposed target or libc gate drifted'
}
if ($contract.desktop.toolkitProposed -ne 'GTK4' -or $contract.desktop.mime -ne 'application/vnd.sico.sapp' -or $contract.desktop.extension -ne '.sapp' -or $contract.desktop.openFieldCode -ne '%f' -or $contract.desktop.invokesShell -or -not $contract.desktop.mustNotSetDefaultSilently) {
    throw 'Linux desktop integration contract drifted'
}
if (@($contract.implementationOrder).Count -ne 8 -or $contract.implementationOrder[0] -ne 'split-linux-artifact-generator' -or $contract.implementationOrder[1] -ne 'xdg-default-path-and-trust-runtime-config') {
    throw 'Linux implementation order no longer begins with the audited blockers'
}

foreach ($needle in 'contract-verified / not-compile-verified / not-runtime-verified','platform-artifacts','open %f','x86_64-unknown-linux-gnu','aarch64-unknown-linux-gnu','XDG_RUNTIME_DIR','copy-before-verify','GtkApplicationWindow','Wayland','X11','SO_PEERCRED','process group','GLIBC','Flatpak','Orca','runtime-verified') {
    if (-not $handbook.Contains($needle)) { throw "Linux handbook missing invariant: $needle" }
}
if (([regex]::Matches($handbook, 'https://')).Count -lt 20) {
    throw 'Linux handbook official reference coverage is unexpectedly low'
}
if (-not $index.Contains('LINUX-DEVELOPMENT.md')) { throw 'platform documentation index does not include Linux' }
if ($step -notmatch '(?m)^> - status: complete\r?$' -or -not $step.Contains('steps kept their allocated meanings and are now closed locally')) {
    throw 'STEP-0071 record is incomplete or changes sequential M7 work'
}

if (-not $platform.Contains('Exec=sico-desktop-host open %f') -or -not $platform.Contains('[Added Associations]') -or -not $platform.Contains('application/vnd.sico.sapp')) {
    throw 'current generated Linux association contract changed; re-audit the handbook'
}
if (-not $hostSource.Contains('association executable must be an .exe') -or -not $hostSource.Contains('native permission dialog is unavailable') -or -not $hostSource.Contains('native UI preview is unavailable')) {
    throw 'current Linux implementation gaps changed; re-audit the handbook status'
}

$requiredOpenArgs = @('Arg::new("runtime")','Arg::new("store")','Arg::new("trusted-key")')
foreach ($needle in $requiredOpenArgs) {
    if (-not $hostSource.Contains($needle)) { throw "current open CLI contract changed: $needle" }
}

$links = ([regex]::Matches($handbook, 'https://')).Count
Write-Output "STEP_0071_OK linux=contract-verified-not-runtime-verified links=$links blockers=artifact-generator,xdg-open-config sequence=closed-local next=external-evidence-or-STEP-0072"
