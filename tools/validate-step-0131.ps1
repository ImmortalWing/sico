# Validates the STEP-0131 machine-readable application-profile matrix:
#  1. the JSON fixture parses and its schema/step/rfcs are current;
#  2. every map/set intrinsic named in the fixture is accepted by the
#     compiler's registry (sico_ir::intrinsic_signature) and rejected when
#     outside the frozen instantiation surface;
#  3. the end-to-end map/set fixture still exists.
# Run from the repository root:  powershell -File tools/validate-step-0131.ps1
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$fixture = Join-Path $repo 'tests/language-matrix/application-profile-v0.json'

$json = Get-Content $fixture -Raw | ConvertFrom-Json
if ($json.schema -ne 'sico.application-profile.v0') { throw 'unexpected schema id' }
if ($json.step -notmatch '^STEP-0131\.\.[0-9]+$') { throw "unexpected step: $($json.step)" }
if ($json.rfc -notlike '*RFC-0038*') { throw "unexpected rfc: $($json.rfc)" }

$fixturePath = Join-Path $repo $json.'exit-corpus'.'map-set-fixture'
if (-not (Test-Path $fixturePath)) { throw "map/set fixture missing: $fixturePath" }
$runnerTest = Join-Path $repo $json.'exit-corpus'.'runner-test'
if (-not (Test-Path $runnerTest)) { throw "runner test missing: $runnerTest" }

# STEP-0143 (RFC-0039): the module-slice fixtures must exist and the
# modules entry must declare the executable check/build/run triple.
if ($json.step -notmatch '01(4[3-9]|7[0])$') {
    throw "matrix step does not include the modules slice: $($json.step)"
}

# STEP-0147 (RFC-0039 section 2.3/2.4): the package/user-WIT slice must be
# executable end to end and the refusal corpus must exist.
if ($json.step -notmatch '0147|0170') {
    throw "matrix step does not include the packages slice: $($json.step)"
}
$packages = $json.matrix.executable.'packages-user-wit'
if (-not $packages) { throw 'packages-user-wit matrix entry missing' }
if (-not ($packages.check -and $packages.build -and $packages.run)) {
    throw 'packages-user-wit must be executable end to end'
}
$refusals = $json.matrix.refused
foreach ($member in 'package-unknown-or-missing', 'package-version-or-limit',
    'wit-shape-mismatch', 'wit-unsupported-type', 'duplicate-or-colliding-package',
    'export-user-interface', 'unknown-exposed-interface', 'impure-package') {
    if (-not $refusals.$member) { throw "refused matrix entry missing: $member" }
}
$refusalCorpus = Join-Path $repo $json.'exit-corpus'.'packages-refusal-corpus'
if (-not (Test-Path $refusalCorpus)) { throw "packages refusal corpus missing: $refusalCorpus" }
foreach ($member in 'modules-fixture', 'modules-module', 'modules-test-fixture') {
    $path = Join-Path $repo $json.'exit-corpus'.$member
    if (-not (Test-Path $path)) { throw "modules fixture missing: $path" }
}
$modules = $json.matrix.executable.'source-modules-use'
if (-not $modules) { throw 'source-modules-use matrix entry missing' }
if (-not ($modules.check -and $modules.build -and $modules.run)) {
    throw 'source-modules-use must be executable end to end'
}

# Compile the registry probe: every canonical instantiation must resolve in
# the compiler's intrinsic registry; off-surface instantiations must not.
$probe = Join-Path $env:TEMP ("sico-step0131-registry-{0}" -f $PID)
New-Item -ItemType Directory -Force -Path $probe | Out-Null
$probeSource = @'
use sico_ir::intrinsic_signature;

fn resolves(name: &str) -> bool {
    intrinsic_signature(name).is_some()
}

#[test]
fn canonical_map_set_instantiations_resolve() {
    let keys: [&str; 5] = ["Text", "Bytes", "Bool", "I64", "U64"];
    for key in keys {
        assert!(resolves(&format!("sico.set.empty[{key}]")));
        assert!(resolves(&format!("sico.set.add[{key}]")));
        assert!(resolves(&format!("sico.set.has[{key}]")));
        assert!(resolves(&format!("sico.set.length[{key}]")));
        for value in keys {
            assert!(resolves(&format!("sico.map.empty[{key},{value}]")));
            assert!(resolves(&format!("sico.map.put[{key},{value}]")));
            assert!(resolves(&format!("sico.map.has[{key},{value}]")));
            assert!(resolves(&format!("sico.map.length[{key},{value}]")));
            // v0 executable surface: map.get only for fixed-width values.
            if *value == *"I64" || *value == *"U64" {
                assert!(resolves(&format!("sico.map.get[{key},{value}]")));
            } else {
                assert!(!resolves(&format!("sico.map.get[{key},{value}]")));
            }
        }
    }
    // off-surface spellings stay closed
    assert!(!resolves("sico.map.put[Text]"));
    assert!(!resolves("sico.map.put[Text,Foo]"));
    assert!(!resolves("sico.map.put [Text,I64]"));
    assert!(!resolves("sico.map.put[Text,I64,Extra]"));
    assert!(!resolves("sico.nonsense[Text]"));
    assert!(resolves("sico.map.put[Text,I64]"));
    assert!(resolves("sico.set.to_list[Text]"));
    // traversal helpers materialize List[Text] only (check-time restriction
    // in sico-semantics; the IR registry itself accepts any key element)
    assert!(resolves("sico.map.keys[Text,I64]"));
}
'@
Set-Content -Path (Join-Path $probe 'registry_probe.rs') -Value $probeSource -Encoding UTF8

# The registry probe lives in sico-ir's test directory for the validation run.
$probeTarget = Join-Path $repo 'crates/sico-ir/tests/step0131_registry_probe.rs'
Copy-Item -Path (Join-Path $probe 'registry_probe.rs') -Destination $probeTarget -Force
try {
    $cargo = Join-Path $env:USERPROFILE '.cargo/bin/cargo.exe'
    $env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
    $logFile = Join-Path $probe 'cargo.log'
    # Invoke via cmd so cargo's stderr never trips PowerShell's error stream.
    cmd /c "`"$cargo`" test --locked --offline -p sico-ir --test step0131_registry_probe > `"$logFile`" 2>&1"
    $code = $LASTEXITCODE
    Get-Content $logFile | ForEach-Object { Write-Host $_ }
    if ($code -ne 0) { throw 'registry probe failed' }

} finally {
    Remove-Item -Force $probeTarget
    Remove-Item -Recurse -Force $probe
}

Write-Host 'validate-step-0131: OK (matrix fixture, corpus paths, registry surface)'
