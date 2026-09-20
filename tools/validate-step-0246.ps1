param([string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot))

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$sico = Join-Path $root 'target\debug\sico.exe'
$runner = Join-Path $root 'runner\sico-runner\target\debug\sico-runner.exe'

foreach ($tool in @($sico, $runner)) {
    if (-not (Test-Path -LiteralPath $tool -PathType Leaf)) {
        throw "STEP-0246 requires a built local tool: $tool"
    }
}

# Windows PowerShell 5.1 (.NET Framework) has neither SHA256.HashData,
# [Convert]::ToHexString, ProcessStartInfo.ArgumentList nor
# StandardInputEncoding; use equivalents that behave identically on 5.1 and 7.
function Get-Sha256Hex([byte[]]$Bytes) {
    $sha256 = [Security.Cryptography.SHA256]::Create()
    try {
        $digest = $sha256.ComputeHash($Bytes)
    } finally {
        [void]$sha256.Dispose()
    }
    $builder = [Text.StringBuilder]::new($digest.Length * 2)
    foreach ($byte in $digest) {
        [void]$builder.Append($byte.ToString('x2'))
    }
    return $builder.ToString()
}

function Get-CanonicalSourceBytes([string]$Path) {
    $bytes = [IO.File]::ReadAllBytes($Path)
    $text = [Text.UTF8Encoding]::new($false, $true).GetString($bytes)
    $canonical = $text.Replace("`r`n", "`n")
    if ($canonical.Contains("`r")) {
        throw "non-CRLF carriage return in $Path"
    }
    return [Text.UTF8Encoding]::new($false).GetBytes($canonical)
}

function Invoke-Process {
    param(
        [string]$FileName,
        [string[]]$Arguments,
        [AllowNull()][string]$StdinText,
        [string]$WorkingDirectory = $root
    )
    $parts = foreach ($argument in $Arguments) {
        if ($argument -match '[\s"]') {
            '"' + ($argument -replace '"', '\"') + '"'
        } else {
            $argument
        }
    }
    $start = [Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $FileName
    $start.WorkingDirectory = $WorkingDirectory
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.RedirectStandardInput = $true
    $start.Arguments = $parts -join ' '
    $start.StandardOutputEncoding = [Text.UTF8Encoding]::new($false)
    $start.StandardErrorEncoding = [Text.UTF8Encoding]::new($false)
    $process = [Diagnostics.Process]::new()
    $process.StartInfo = $start
    [void]$process.Start()
    if ($null -ne $StdinText) {
        # Raw UTF-8 bytes through the pipe base stream: Windows PowerShell 5.1
        # has no StandardInputEncoding and would re-encode through the console
        # codepage, corrupting non-ASCII source text.
        $stdinBytes = [Text.UTF8Encoding]::new($false).GetBytes($StdinText)
        $process.StandardInput.BaseStream.Write($stdinBytes, 0, $stdinBytes.Length)
        $process.StandardInput.BaseStream.Flush()
        $process.StandardInput.BaseStream.Close()
    } else {
        $process.StandardInput.Close()
    }
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    return [pscustomobject]@{
        ExitCode = $process.ExitCode
        Stdout = $stdout
        Stderr = $stderr
    }
}

$manifestPath = Join-Path $root 'selfhost\corpus-v0.json'
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.entry_count -ne 215 -or $manifest.entries.Count -ne 215) {
    throw 'M22 corpus cardinality drifted'
}
foreach ($entry in $manifest.entries) {
    [byte[]]$bytes = Get-CanonicalSourceBytes (Join-Path $root $entry.path)
    if ($entry.bytes -ne $bytes.Length) {
        throw "canonical source length drifted: $($entry.path)"
    }
    if ($entry.source_sha256 -cne (Get-Sha256Hex $bytes)) {
        throw "canonical source digest drifted: $($entry.path)"
    }
}

& (Join-Path $root 'tools\update-m22-corpus.ps1') -RepositoryRoot $root -Verify
if ($LASTEXITCODE -ne 0) { throw 'M22 corpus updater verify failed' }

$compiler = Get-Content -LiteralPath (Join-Path $root 'selfhost\compiler.sico') -Raw
$parser = Get-Content -LiteralPath (Join-Path $root 'selfhost\parser.sico') -Raw
$checkerTest = Get-Content -LiteralPath (Join-Path $root 'runner\sico-runner\tests\selfhost_checker.rs') -Raw
$updater = Get-Content -LiteralPath (Join-Path $root 'tools\update-m22-corpus.ps1') -Raw

foreach ($needle in @(
    'use parser.lex_words',
    'use parser.scalar_ir',
    'parser.scalar_ir(general_words, input.stdin)'
)) {
    if (-not $compiler.Contains($needle)) { throw "unified compiler lowering missing: $needle" }
}
foreach ($forbidden in @(
    '"source_name":"identity.sico"',
    'function identity_source(',
    'function add_source(',
    'unsupported self-host source shape'
)) {
    if ($compiler.Contains($forbidden)) { throw "exact-source fallback remains: $forbidden" }
}
if (-not $parser.StartsWith("module parser`n") -and -not $parser.StartsWith("module parser`r`n")) {
    throw 'parser is not a reusable module'
}
if (-not $checkerTest.Contains('Sha256::digest(&source)')) {
    throw 'checker corpus test does not verify source SHA-256'
}
if (-not $updater.Contains('$canonical = $text.Replace("`r`n", "`n")')) {
    throw 'corpus updater does not canonicalize LF bytes'
}

$temp = Join-Path ([IO.Path]::GetTempPath()) ("sico-step0246-" + [Guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $temp)
try {
    $parserComponent = Join-Path $temp 'parser.component.wasm'
    $compilerComponent = Join-Path $temp 'compiler.component.wasm'
    foreach ($build in @(
        @($parserComponent, (Join-Path $root 'selfhost\parser_driver.sico')),
        @($compilerComponent, (Join-Path $root 'selfhost\compiler.sico'))
    )) {
        $result = Invoke-Process $sico @(
            'build', '--profile', 'script-v0', '--output', $build[0], $build[1]
        ) $null
        if ($result.ExitCode -ne 0) {
            throw "Sico self-host component build failed: $($result.Stderr)"
        }
    }

    $identity = "function identity(value: Int) returns Int:`n  return value`nend function`n"
    $parserRun = Invoke-Process $runner @(
        '--fuel', '5000000000', $parserComponent, '--', '--emit-ir'
    ) $identity
    if ($parserRun.ExitCode -ne 0) { throw "parser driver failed: $($parserRun.Stderr)" }
    $parserIr = $parserRun.Stdout | ConvertFrom-Json
    if ($parserIr.source_name -cne 'selfhost-input.sico' -or $parserIr.functions[0].name -cne 'identity') {
        throw 'parser driver canonical identity IR drifted'
    }

    $compilerRun = Invoke-Process $runner @(
        '--fuel', '5000000000', $compilerComponent
    ) $identity
    if ($compilerRun.ExitCode -ne 0) { throw "compiler identity failed: $($compilerRun.Stderr)" }
    $compilerIr = $compilerRun.Stdout | ConvertFrom-Json
    if ($compilerIr.source_name -cne 'selfhost-input.sico' -or $compilerIr.functions[0].name -cne 'identity') {
        throw 'compiler canonical identity IR drifted'
    }

    $add = "function add(a: Int, b: Int) returns Int:`n  return a + b`nend function`n"
    $addRun = Invoke-Process $runner @(
        '--fuel', '5000000000', $compilerComponent
    ) $add
    if ($addRun.ExitCode -ne 0) { throw "compiler add failed: $($addRun.Stderr)" }
    $addIr = $addRun.Stdout | ConvertFrom-Json
    $instruction = $addIr.functions[0].blocks[0].instructions[0]
    if ($instruction.operation.op -cne 'add_int' -or
        $instruction.operation.data.left -ne 0 -or
        $instruction.operation.data.right -ne 1) {
        throw 'compiler add_int IR drifted'
    }

    $constant = "function answer() returns Int:`n  return 42`nend function`n"
    $coreRun = Invoke-Process $runner @(
        '--fuel', '5000000000', $compilerComponent, '--', '--emit-core-hex'
    ) $constant
    if ($coreRun.ExitCode -ne 0 -or -not $coreRun.Stdout.StartsWith('0061736d01000000')) {
        throw "bounded Core-Wasm encoder failed: $($coreRun.Stderr)"
    }
}
finally {
    Remove-Item -LiteralPath $temp -Recurse -Force
}

Push-Location $root
try {
    git diff --check
    if ($LASTEXITCODE -ne 0) { throw 'git diff --check failed' }
}
finally {
    Pop-Location
}

Write-Output 'STEP_0246_OK canonical-corpus=215 unified-lowering=identity+add_int core-seam=constant'
