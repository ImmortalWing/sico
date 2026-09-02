param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) { $cargo = (Get-Command cargo -ErrorAction Stop).Source }
. (Join-Path $root 'tools\lib\native-command.ps1')

# --- toolchain hygiene ---
Invoke-NativeChecked $cargo @('fmt', '--all', '--check') 'cargo fmt check failed'
Invoke-NativeChecked $cargo @('clippy', '--workspace', '--all-targets', '--offline', '--locked', '--', '-D', 'warnings') 'clippy failed'

# --- provider crate tests: deterministic CA, valid chain accept, wrong
# --- hostname / untrusted issuer / malformed PEM refuse before connect ---
Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-http-provider') 'tls-provider-tests-failed|STEP-0111/0112'

# --- module boundaries cover the new package ---
& (Join-Path $root 'tools\validate-module-boundaries.ps1') -RepositoryRoot $root
if (-not $?) { throw 'module-boundary validation failed' }
$boundary = (& (Join-Path $root 'tools\validate-module-boundaries.ps1') -RepositoryRoot $root) -join ''
if ($boundary -notlike '*packages=27*') { throw "expected 27 packages: $boundary" }

# --- RFC decisions frozen ---
$rfc = Get-Content (Join-Path $root 'docs\rfc\RFC-0037-secure-http-provider-v0.md') -Raw -Encoding UTF8
foreach ($needle in @(
        'sico:script/http@0.2.0', 'scheme + canonical host + effective port', 'rustls 0.23',
        'system-roots', 'pinned-roots', 'cannot be disabled by guest code', 'no wildcard grants in v1',
        'header-only', '64 MiB', '1 GiB', 'never crosses a Store or generation'
    )) {
    if ($rfc -notlike "*$needle*") { throw "RFC-0037 decision missing: $needle" }
}
if ($rfc -notlike '*TLS_ROUNDTRIP_OK*') { throw 'RFC-0037 dependency evidence missing' }

# --- no insecure fallback in source ---
$source = (Get-Content (Join-Path $root 'crates\sico-http-provider\src\lib.rs') -Raw -Encoding UTF8) -replace '\s+', ' '
foreach ($forbidden in @('danger()', 'with_custom_certificate_verifier', 'danger_accept_invalid_certs', 'InsecureSkipVerify')) {
    if ($source -like "*$forbidden*") { throw "insecure TLS path present: $forbidden" }
}

# --- security gates: verification-disable and no silent claims ---
& (Join-Path $root 'tools\validate-step-0089.ps1') -RepositoryRoot $root
if (-not $?) { throw 'STEP-0089 HTTP v0 regression under STEP-0111/0112' }

Write-Output 'STEP_0111_0112_OK rfc=accepted tls=valid-accept+3-refusals no-danger-path packages=27 http-v0=green'
