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

# --- toolchain hygiene + new crate tests ---
Invoke-NativeChecked $cargo @('fmt', '--all', '--check') 'cargo fmt check failed'
Invoke-NativeChecked $cargo @('clippy', '--workspace', '--all-targets', '--offline', '--locked', '--', '-D', 'warnings') 'clippy failed'
Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-mcp-server') 'mcp-server-tests-failed|STEP-0121'
Invoke-NativeChecked $cargo @('test', '--offline', '--locked', '-p', 'sico-ai-tools') 'ai-tools-regression-failed|STEP-0121'

# --- module boundary validator now covers 26 packages ---
& (Join-Path $root 'tools\validate-module-boundaries.ps1') -RepositoryRoot $root
if (-not $?) { throw 'module-boundary validation failed (new package assignment)' }
$boundaryOutput = (& (Join-Path $root 'tools\validate-module-boundaries.ps1') -RepositoryRoot $root) -join ''
if ($boundaryOutput -notlike '*packages=26*') { throw "expected 26 workspace packages, got: $boundaryOutput" }

# --- 0068 oracle must stay green (frozen surface unchanged) ---
& (Join-Path $root 'tools\validate-step-0068.ps1') -RepositoryRoot $root
if (-not $?) { throw 'STEP-0068 regression under STEP-0121' }

# --- AIT-025+ contract cases registered ---
$cases = Get-Content (Join-Path $root 'ai-eval\tooling-cases.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if (@($cases.cases).Count -ne 29) { throw "expected 29 tooling cases, found $(@($cases.cases).Count)" }
foreach ($id in 'AIT-025', 'AIT-026', 'AIT-027', 'AIT-028', 'AIT-029') {
    if (@($cases.cases | Where-Object id -eq $id).Count -ne 1) { throw "missing contract case: $id" }
}

# --- fail-closed invariants present in the new crate source ---
$source = (Get-Content (Join-Path $root 'crates\sico-mcp-server\src\lib.rs') -Raw -Encoding UTF8) -replace '\s+', ' '
foreach ($needle in @(
        'sico.ai-tool.response.v0', 'no filesystem writes', 'no process launches', 'no network',
        'five_hundred_twelve_protocol_mutations_fail_closed_through_mcp',
        'real_stdio_session_performs_inspect_then_validate_fix_roundtrip', '2025-06-18'
    )) {
    if ($source -notlike "*$needle*") { throw "MCP crate invariant missing: $needle" }
}
$manifest = Get-Content (Join-Path $root 'crates\sico-mcp-server\Cargo.toml') -Raw -Encoding UTF8
if ($manifest -match 'tokio|rmcp|reqwest|hyper') { throw 'MCP crate must stay dependency-minimal (no runtime/network stack)' }

Write-Output 'STEP_0121_OK mcp-stdio=4-tools schema=registered mutations-512=fail-closed-through-mcp session=roundtrip packages=26 ai-tools=regression-green cases=29'
