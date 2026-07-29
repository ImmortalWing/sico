$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$cargo = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
if (-not (Test-Path -LiteralPath $cargo)) {
    $cargo = (Get-Command cargo -ErrorAction Stop).Source
}
$env:CARGO_REGISTRIES_CRATES_IO_PROTOCOL = 'sparse'
$env:RUSTUP_TOOLCHAIN = 'stable-x86_64-pc-windows-gnu'
. (Join-Path $root 'tools\lib\native-command.ps1')

Push-Location $root
try {
    Invoke-NativeChecked $cargo @(
        'test', '--offline', '--locked',
        '-p', 'sico-tooling-protocol', '-p', 'sico-language-server',
        '-p', 'sico-ai-tools'
    ) 'editor-ai-feedback-tests-failed|STEP-0101'

    $schema = Get-Content tooling\schema\debug-launch-plan-v0.schema.json -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($schema.properties.schema.const -ne 'sico.debug-launch-plan.v0') { throw 'debug-plan-schema-mismatch' }
    if ($schema.properties.shell.const -ne $false) { throw 'debug-plan-shell-must-be-false' }
    if ($schema.properties.executable.const -ne 'sico-dap') { throw 'debug-plan-adapter-mismatch' }

    $lsp = Get-Content crates\sico-language-server\src\lib.rs -Raw -Encoding UTF8
    foreach ($required in @('sico.debug', 'debug_launch_plan', 'debugMap', 'debugIdentity', 'documentId')) {
        if (-not $lsp.Contains($required)) { throw "lsp-debug-plan-missing|$required" }
    }
    $ai = Get-Content crates\sico-ai-tools\src\lib.rs -Raw -Encoding UTF8
    foreach ($required in @('SummarizeExecution', 'validate_execution_event', 'validate_runtime_fault', 'data-only')) {
        if (-not $ai.Contains($required)) { throw "ai-summary-boundary-missing|$required" }
    }
} finally {
    Pop-Location
}

Write-Output 'STEP_0101_OK debug-plan=sico.debug-launch-plan.v0 argv=direct shell=false adapter=sico-dap lsp=data-only ai=plan+redacted-summary events=identity+sequence+terminal fault=typed-no-message authority=no-launch next=STEP-0102'
