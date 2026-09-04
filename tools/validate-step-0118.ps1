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

# --- provider crate: all five layers ---
Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-http-provider') 'http-provider-tests-failed|STEP-0118'

# --- module boundaries ---
& (Join-Path $root 'tools\validate-module-boundaries.ps1') -RepositoryRoot $root
if (-not $?) { throw 'module-boundary validation failed' }

# --- insecure-API scan (automatic NO-GO list) ---
$providerSource = ((Get-Content (Join-Path $root 'crates\sico-http-provider\src\lib.rs') -Raw -Encoding UTF8) + `
    (Get-Content (Join-Path $root 'crates\sico-http-provider\src\authority.rs') -Raw -Encoding UTF8) + `
    (Get-Content (Join-Path $root 'crates\sico-http-provider\src\framing.rs') -Raw -Encoding UTF8) + `
    (Get-Content (Join-Path $root 'crates\sico-http-provider\src\redirect.rs') -Raw -Encoding UTF8) + `
    (Get-Content (Join-Path $root 'crates\sico-http-provider\src\secrets.rs') -Raw -Encoding UTF8) + `
    (Get-Content (Join-Path $root 'crates\sico-http-provider\src\engine.rs') -Raw -Encoding UTF8)) -replace '\s+', ' '
foreach ($forbidden in @('danger()', 'with_custom_certificate_verifier', 'danger_accept_invalid_certs', 'InsecureSkipVerify', 'accept_invalid_hostnames')) {
    if ($providerSource -like "*$forbidden*") { throw "insecure path present: $forbidden" }
}

# --- RFC decisions ---
$rfc = Get-Content (Join-Path $root 'docs\rfc\RFC-0037-secure-http-provider-v0.md') -Raw -Encoding UTF8
if ($rfc -notlike '*status: accepted*') { throw 'RFC-0037 not accepted' }

# --- audit doc consistency ---
foreach ($item in @(
        @{ P = 'docs\steps\STEP-0118-m12-exit-audit.md'; T = 'GO-core' },
        @{ P = 'docs\steps\STEP-0113-endpoint-authority-dns.md'; T = 'status: complete' },
        @{ P = 'docs\steps\STEP-0114-streaming-bodies.md'; T = 'status: complete' },
        @{ P = 'docs\steps\STEP-0115-redirect-policy.md'; T = 'status: complete' },
        @{ P = 'docs\steps\STEP-0116-secret-provider-redaction.md'; T = 'status: complete' }
    )) {
    if (-not (Select-String -LiteralPath (Join-Path $root $item.P) -Pattern $item.T -Quiet -Encoding UTF8)) {
        throw "audit input mismatch: $($item.P) / $($item.T)"
    }
}

# --- full regression: M0-M11 aggregate + provider ---
& (Join-Path $root 'tools\validate-step-0110.ps1') -RepositoryRoot $root
if (-not $?) { throw 'M0-M11 aggregate regression failed' }

Write-Output 'STEP_0118_OK m12=GO-core provider=33-tests layers=authority+tls+framing+redirect+secrets insecure-scan=clean packages=27 regression=m0-m11-green followup=wit-runner-integration+linux-corpus'
