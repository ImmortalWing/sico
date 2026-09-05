# STEP-0125 validator: guest-visible http@0.2.0 runner integration.
# Runs the provider corpus, the runner corpus (serially for the RSS-bound
# DAP tests), the frozen STEP-0089 oracle, module boundaries and the
# insecure-API scan.

$ErrorActionPreference = "Stop"
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$env:RUSTUP_TOOLCHAIN = "1.98.0-x86_64-pc-windows-gnu"
$failures = @()

function Invoke-Step {
    param([string]$Name, [scriptblock]$Action)
    Write-Host "==> $Name"
    & $Action
    if ($LASTEXITCODE -ne 0) {
        Write-Host "    FAILED: $Name"
        $script:failures += $Name
    } else {
        Write-Host "    ok"
    }
}

Invoke-Step "fmt (workspace)" { & $cargo fmt --all -- --check }
Invoke-Step "fmt (runner)" { & $cargo fmt --manifest-path runner\sico-runner\Cargo.toml -- --check }

Invoke-Step "clippy (workspace, -D warnings)" {
    & $cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
}
Invoke-Step "clippy (runner, -D warnings)" {
    & $cargo clippy --locked --offline --manifest-path runner\sico-runner\Cargo.toml --all-targets -- -D warnings
}

Invoke-Step "provider tests (48)" {
    & $cargo test --locked --offline -p sico-http-provider
}

Invoke-Step "runner tests (lib + http2 + runner, serial)" {
    & $cargo test --locked --offline --manifest-path runner\sico-runner\Cargo.toml -- --test-threads=1
}

Invoke-Step "STEP-0089 buffered http oracle rerun" {
    & $cargo test --locked --offline -p sico-package --test package
    & $cargo test --locked --offline -p sico-cli --test cli
}

Invoke-Step "module boundaries (27 root packages)" {
    & .\tools\validate-module-boundaries.ps1
}

Invoke-Step "insecure-API scan (provider)" {
    $hits = Select-String -Path crates\sico-http-provider\src\*.rs `
        -Pattern "danger_|disable_verification|InsecureSkip|native-tls" `
        -CaseSensitive:$false
    if ($hits) { $hits | ForEach-Object { Write-Host $_.Line }; exit 1 }
    exit 0
}

Invoke-Step "RFC-0037 decision presence" {
    if (-not (Test-Path docs\rfc\RFC-0037-secure-http-provider-v0.md)) { exit 1 }
    if (-not (Test-Path wit\script-http2-v0\http2.wit)) { exit 1 }
    if (-not (Test-Path docs\steps\STEP-0125-http2-runner-integration.md)) { exit 1 }
    exit 0
}

if ($failures.Count -gt 0) {
    Write-Host "FAILED: $($failures -join ', ')"
    exit 1
}
Write-Host "STEP-0125 validator: ALL GREEN"
exit 0
