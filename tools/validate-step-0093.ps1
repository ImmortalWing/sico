param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
[Console]::OutputEncoding = [Text.Encoding]::UTF8
$OutputEncoding = [Text.UTF8Encoding]::new($false)

cargo test --offline --locked -p sico-tooling-protocol -p sico-language-server -p sico-ai-tools
if ($LASTEXITCODE -ne 0) { throw 'STEP-0093 tooling tests failed' }
cargo build --offline --locked -p sico-language-server -p sico-ai-tools
if ($LASTEXITCODE -ne 0) { throw 'STEP-0093 tooling build failed' }
& (Join-Path $root 'tools\validate-module-boundaries.ps1')

$schema = Get-Content (Join-Path $root 'tooling\schema\execution-plan-v0.schema.json') -Raw -Encoding UTF8 | ConvertFrom-Json
if ($schema.properties.schema.const -ne 'sico.execution-plan.v0' -or
    $schema.properties.shell.const -ne $false -or
    $schema.properties.output.properties.capture_limit_bytes.maximum -ne 1048576 -or
    $schema.properties.source_map.properties.debug_adapter.const -ne $false) {
    throw 'execution plan schema does not freeze safety/debug bounds'
}

function Invoke-JsonProcess([string]$executable, [string]$stdinText) {
    $psi = [Diagnostics.ProcessStartInfo]::new()
    $psi.FileName = $executable
    $psi.UseShellExecute = $false
    $psi.RedirectStandardInput = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($psi)
    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $process.StandardInput.Write($stdinText)
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(10000)) {
        & taskkill.exe /PID $process.Id /T /F | Out-Null
        $process.WaitForExit()
        throw "tooling process timed out: $executable"
    }
    return [pscustomobject]@{
        Exit = $process.ExitCode
        Stdout = $stdoutTask.GetAwaiter().GetResult()
        Stderr = $stderrTask.GetAwaiter().GetResult()
    }
}

# Real AI-tool binary: data-only plan, literal metacharacters, bounded capture.
$aiRequest = @{
    schema = 'sico.ai-tool.request.v0'
    protocol_version = 0
    request_id = 'step-0093-ai'
    operation = 'plan_execution'
    budget = @{
        max_files = 16
        max_input_bytes = 1048576
        max_symbols = 256
        max_diagnostics = 100
        max_response_bytes = 1048576
    }
    input = @{
        mode = 'run'
        program = 'path with spaces\$(literal);app.sico'
        arguments = @('; echo nope', '$(whoami)')
        max_log_bytes = 65536
    }
} | ConvertTo-Json -Depth 8 -Compress
$ai = Invoke-JsonProcess (Join-Path $root 'target\debug\sico-ai-tool.exe') $aiRequest
$aiResult = $ai.Stdout | ConvertFrom-Json
if ($ai.Exit -ne 0 -or $aiResult.ok -ne $true -or $aiResult.result.shell -ne $false -or
    $aiResult.result.output.capture_limit_bytes -ne 65536 -or
    $aiResult.result.arguments[2] -cne 'path with spaces\$(literal);app.sico' -or
    $aiResult.result.source_map.runtime_locations -ne $false) {
    throw "AI execution plan mismatch: $($ai.Stdout)"
}

# Real LSP binary over Content-Length framing: run/watch/repl plans and honest debug refusal.
$messages = @(
    @{ jsonrpc = '2.0'; id = 1; method = 'initialize'; params = @{} },
    @{ jsonrpc = '2.0'; id = 2; method = 'workspace/executeCommand'; params = @{ command = 'sico.run'; arguments = @('path with spaces\$(literal);app.sico') } },
    @{ jsonrpc = '2.0'; id = 3; method = 'workspace/executeCommand'; params = @{ command = 'sico.watch'; arguments = @('watch.sico') } },
    @{ jsonrpc = '2.0'; id = 4; method = 'workspace/executeCommand'; params = @{ command = 'sico.repl'; arguments = @() } },
    @{ jsonrpc = '2.0'; id = 5; method = 'workspace/executeCommand'; params = @{ command = 'sico.debug'; arguments = @('app.sico') } },
    @{ jsonrpc = '2.0'; id = 6; method = 'shutdown'; params = @{} },
    @{ jsonrpc = '2.0'; method = 'exit'; params = @{} }
)
$framed = ''
foreach ($message in $messages) {
    $body = $message | ConvertTo-Json -Depth 8 -Compress
    $length = [Text.Encoding]::UTF8.GetByteCount($body)
    $framed += "Content-Length: $length`r`n`r`n$body"
}
$lsp = Invoke-JsonProcess (Join-Path $root 'target\debug\sico-lsp.exe') $framed
if ($lsp.Exit -ne 0 -or ([regex]::Matches($lsp.Stdout, 'sico.execution-plan.v0')).Count -ne 3 -or
    ([regex]::Matches($lsp.Stdout, '"shell":false')).Count -ne 3 -or
    $lsp.Stdout -notmatch 'terminate-direct-child-tree' -or
    $lsp.Stdout -notmatch '"code":-32004') {
    throw "LSP execution protocol mismatch: $($lsp.Stdout) $($lsp.Stderr)"
}

Write-Output 'STEP_0093_OK schema=sico.execution-plan.v0 lsp=run,watch,repl ai=plan_execution shell=false logs=1MiB-bounded cancel=client-tree source-map=compile-only debug=honest-refusal'
