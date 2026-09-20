param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
    [switch]$Verify
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$root = (Resolve-Path $RepositoryRoot).Path
$sico = Join-Path $root 'target\debug\sico.exe'
if (-not (Test-Path -LiteralPath $sico)) {
    throw "build sico before refreshing the M22 corpus: $sico"
}

# Windows PowerShell 5.1 (.NET Framework) has neither ProcessStartInfo.ArgumentList
# nor StandardInputEncoding; quote into .Arguments instead.  Redirected stdin then
# uses the console encoding, which is safe for the ASCII-only invocations here.
function ConvertTo-ArgumentsString {
    param([string[]]$Arguments)
    $parts = foreach ($argument in $Arguments) {
        if ($argument -match '[\s"]') {
            '"' + ($argument -replace '"', '\"') + '"'
        } else {
            $argument
        }
    }
    return ($parts -join ' ')
}

function Invoke-Sico {
    param(
        [string[]]$Arguments,
        [AllowNull()][string]$StdinText
    )
    $start = [System.Diagnostics.ProcessStartInfo]::new()
    $start.FileName = $sico
    $start.WorkingDirectory = $root
    $start.UseShellExecute = $false
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $start.RedirectStandardInput = $true
    $start.StandardOutputEncoding = [System.Text.UTF8Encoding]::new($false)
    $start.StandardErrorEncoding = [System.Text.UTF8Encoding]::new($false)
    $start.Arguments = ConvertTo-ArgumentsString $Arguments
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    [void]$process.Start()
    if ($null -ne $StdinText) {
        # Write raw UTF-8 bytes through the pipe base stream: Windows
        # PowerShell 5.1 has no StandardInputEncoding and would otherwise
        # re-encode through the console codepage, corrupting non-ASCII source.
        $stdinBytes = [System.Text.UTF8Encoding]::new($false).GetBytes($StdinText)
        $process.StandardInput.BaseStream.Write($stdinBytes, 0, $stdinBytes.Length)
        $process.StandardInput.BaseStream.Flush()
        $process.StandardInput.BaseStream.Close()
    } else {
        $process.StandardInput.Close()
    }
    $stdout = $process.StandardOutput.ReadToEnd()
    $stderr = $process.StandardError.ReadToEnd()
    $process.WaitForExit()
    [pscustomobject]@{
        ExitCode = $process.ExitCode
        Stdout = $stdout.Replace("`r`n", "`n")
        Stderr = $stderr.Replace("`r`n", "`n")
    }
}

function Get-Sha256Hex {
    param([byte[]]$Bytes)
    # [Security.Cryptography.SHA256]::HashData and [Convert]::ToHexString need
    # .NET 5+; SHA256.Create().ComputeHash plus a hex loop runs on Windows
    # PowerShell 5.1 and PowerShell 7 alike with identical output.
    $sha256 = [System.Security.Cryptography.SHA256]::Create()
    try {
        $digest = $sha256.ComputeHash($Bytes)
    } finally {
        [void]$sha256.Dispose()
    }
    $builder = [System.Text.StringBuilder]::new($digest.Length * 2)
    foreach ($byte in $digest) {
        [void]$builder.Append($byte.ToString('x2'))
    }
    return $builder.ToString()
}

function Get-CanonicalSourceBytes {
    param([string]$Path)
    $bytes = [System.IO.File]::ReadAllBytes($Path)
    $text = [System.Text.UTF8Encoding]::new($false, $true).GetString($bytes)
    $canonical = $text.Replace("`r`n", "`n")
    if ($canonical.Contains("`r")) {
        throw "source contains a non-CRLF carriage return: $Path"
    }
    return [System.Text.UTF8Encoding]::new($false).GetBytes($canonical)
}

# ConvertTo-Json formatting differs between Windows PowerShell 5.1 and
# PowerShell 7, which would make the frozen manifests host-dependent (the
# exact defect the canonical-LF re-freeze closed).  Serialize deterministically
# instead: 2-space indent, no-BOM UTF-8, identical bytes on both hosts.
function ConvertTo-CanonicalJsonString {
    param([string]$Text)
    $builder = [System.Text.StringBuilder]::new($Text.Length + 2)
    [void]$builder.Append('"')
    foreach ($character in $Text.ToCharArray()) {
        switch ([int]$character) {
            34 { [void]$builder.Append('\"') }
            92 { [void]$builder.Append('\\') }
            8 { [void]$builder.Append('\b') }
            12 { [void]$builder.Append('\f') }
            10 { [void]$builder.Append('\n') }
            13 { [void]$builder.Append('\r') }
            9 { [void]$builder.Append('\t') }
            default {
                if ([int]$character -lt 32) {
                    [void]$builder.Append(('\u{0:x4}' -f [int]$character))
                } else {
                    [void]$builder.Append($character)
                }
            }
        }
    }
    [void]$builder.Append('"')
    return $builder.ToString()
}

function ConvertTo-CanonicalJson {
    param($Value, [int]$Depth = 0)
    $indent = ' ' * (2 * $Depth)
    $childIndent = ' ' * (2 * ($Depth + 1))
    if ($null -eq $Value) { return 'null' }
    if ($Value -is [bool]) { if ($Value) { return 'true' } else { return 'false' } }
    if ($Value -is [System.SByte] -or $Value -is [System.Byte] -or $Value -is [System.Int16] -or
        $Value -is [System.UInt16] -or $Value -is [System.Int32] -or $Value -is [System.UInt32] -or
        $Value -is [System.Int64] -or $Value -is [System.UInt64]) {
        return $Value.ToString([System.Globalization.CultureInfo]::InvariantCulture)
    }
    if ($Value -is [string]) { return (ConvertTo-CanonicalJsonString $Value) }
    if ($Value -is [System.Collections.IDictionary]) {
        if ($Value.Count -eq 0) { return '{}' }
        $lines = foreach ($key in $Value.Keys) {
            $childIndent + (ConvertTo-CanonicalJsonString ([string]$key)) + ': ' +
                (ConvertTo-CanonicalJson $Value[$key] ($Depth + 1))
        }
        return "{`n" + ($lines -join ",`n") + "`n" + $indent + '}'
    }
    if ($Value -is [System.Collections.IEnumerable]) {
        $items = @($Value)
        if ($items.Count -eq 0) { return '[]' }
        $lines = foreach ($item in $items) {
            $childIndent + (ConvertTo-CanonicalJson $item ($Depth + 1))
        }
        return "[`n" + ($lines -join ",`n") + "`n" + $indent + ']'
    }
    throw "unsupported manifest value type: $($Value.GetType().FullName)"
}

$relativePaths = @(
    & rg --files syntax-candidates semantic-cases tests/end-to-end -g '*.sico'
) | ForEach-Object { $_.Replace('\', '/') }
# Sort-Object ordering is culture-sensitive and differs between hosts; the
# frozen manifest requires byte-wise (ordinal) path order everywhere.
$relativePaths = @($relativePaths)
[System.Array]::Sort($relativePaths, [System.StringComparer]::Ordinal)

$artifactRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("sico-m22-corpus-{0}" -f [Guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $artifactRoot)

$entryIndex = 0
$buildEntries = @()
$entries = foreach ($relativePath in $relativePaths) {
    $nativePath = Join-Path $root $relativePath
    # Repository sources are canonical LF (`.gitattributes`).  Hash the
    # canonical bytes rather than the host checkout's autocrlf materialization
    # so the frozen manifest is identical on Windows, Linux and clean CI.
    [byte[]]$bytes = Get-CanonicalSourceBytes $nativePath
    $format = Invoke-Sico -Arguments @('format', $relativePath) -StdinText $null
    $formatState = if ($format.ExitCode -eq 0) { 'accepted' } else { 'refused' }
    $formattedSha256 = $null
    $idempotent = $false
    if ($format.ExitCode -eq 0) {
        $formatted = [System.Text.UTF8Encoding]::new($false).GetBytes($format.Stdout)
        $formattedSha256 = Get-Sha256Hex $formatted
        $second = Invoke-Sico -Arguments @('format', '-') -StdinText $format.Stdout
        $idempotent = $second.ExitCode -eq 0 -and $second.Stdout -ceq $format.Stdout
    }

    $check = Invoke-Sico -Arguments @('check', '--json', $relativePath) -StdinText $null
    $checkState = if ($check.ExitCode -eq 0) { 'accepted' } else { 'refused' }
    $diagnosticText = "$($check.Stdout)`n--stderr--`n$($check.Stderr)"
    $diagnosticBytes = [System.Text.UTF8Encoding]::new($false).GetBytes($diagnosticText)
    $diagnosticIds = @(
        [regex]::Matches($diagnosticText, '\bE\d{4}\b') |
            ForEach-Object { $_.Value } |
            Sort-Object -Unique
    )
    if ($diagnosticIds.Count -eq 0 -and $check.ExitCode -ne 0) {
        $diagnosticLower = $diagnosticText.ToLowerInvariant()
        if ($diagnosticLower.Contains('lexical error')) {
            $diagnosticIds = @('LEXICAL')
        } elseif ($diagnosticLower.Contains('syntax')) {
            $diagnosticIds = @('SYNTAX')
        } else {
            $diagnosticIds = @('UNCLASSIFIED')
        }
    }

    $artifactPath = Join-Path $artifactRoot ("{0:D4}.component.wasm" -f $entryIndex)
    $build = Invoke-Sico -Arguments @(
        'build', '--profile', 'script-v0', '--output', $artifactPath, $relativePath
    ) -StdinText $null
    $buildState = if ($build.ExitCode -eq 0) { 'accepted' } else { 'refused' }
    $artifactSha256 = $null
    $componentReproducible = $false
    if ($build.ExitCode -eq 0) {
        if (-not (Test-Path -LiteralPath $artifactPath)) {
            throw "sico build succeeded without artifact: $relativePath"
        }
        $artifactBytes = [System.IO.File]::ReadAllBytes($artifactPath)
        $artifactSha256 = Get-Sha256Hex $artifactBytes
        $reproPath = Join-Path $artifactRoot ("{0:D4}.repro.component.wasm" -f $entryIndex)
        $repro = Invoke-Sico -Arguments @(
            'build', '--profile', 'script-v0', '--output', $reproPath, $relativePath
        ) -StdinText $null
        if ($repro.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $reproPath)) {
            throw "second deterministic build failed: $relativePath"
        }
        $reproBytes = [System.IO.File]::ReadAllBytes($reproPath)
        # Windows PowerShell 5.1 cannot invoke static generic methods
        # ([Linq.Enumerable]::SequenceEqual[byte] is a parse error there).
        $componentReproducible = $artifactBytes.Length -eq $reproBytes.Length
        if ($componentReproducible) {
            for ($byteIndex = 0; $byteIndex -lt $artifactBytes.Length; $byteIndex++) {
                if ($artifactBytes[$byteIndex] -ne $reproBytes[$byteIndex]) {
                    $componentReproducible = $false
                    break
                }
            }
        }
        if (-not $componentReproducible) {
            throw "script-v0 artifact is not reproducible: $relativePath"
        }
        Remove-Item -LiteralPath $artifactPath -Force
        Remove-Item -LiteralPath $reproPath -Force
    } elseif (Test-Path -LiteralPath $artifactPath) {
        throw "sico build refusal left an artifact: $relativePath"
    }
    $buildDiagnosticText = "$($build.Stdout)`n--stderr--`n$($build.Stderr)"
    $buildDiagnosticText = $buildDiagnosticText.Replace($artifactPath, '<artifact>')
    $buildDiagnosticBytes = [System.Text.UTF8Encoding]::new($false).GetBytes($buildDiagnosticText)
    $buildDiagnosticIds = @(
        [regex]::Matches($buildDiagnosticText, '\bE\d{4}\b') |
            ForEach-Object { $_.Value } |
            Sort-Object -Unique
    )
    if ($buildDiagnosticIds.Count -eq 0 -and $build.ExitCode -ne 0) {
        $buildDiagnosticLower = $buildDiagnosticText.ToLowerInvariant()
        if ($buildDiagnosticLower.Contains('lexical error')) {
            $buildDiagnosticIds = @('LEXICAL')
        } elseif ($buildDiagnosticLower.Contains('script profile requires')) {
            $buildDiagnosticIds = @('SCRIPT_ABI')
        } elseif ($buildDiagnosticLower.Contains('syntax error')) {
            $buildDiagnosticIds = @('SYNTAX')
        } else {
            $buildDiagnosticIds = @('UNCLASSIFIED')
        }
    }

    $buildEntries += [ordered]@{
        path = $relativePath
        source_sha256 = Get-Sha256Hex $bytes
        rust_script_build = $buildState
        rust_script_build_exit = $build.ExitCode
        component_sha256 = $artifactSha256
        component_reproducible = $componentReproducible
        build_diagnostic_ids = $buildDiagnosticIds
        build_diagnostic_sha256 = Get-Sha256Hex $buildDiagnosticBytes
    }

    [ordered]@{
        path = $relativePath
        bytes = $bytes.Length
        source_sha256 = Get-Sha256Hex $bytes
        rust_format = $formatState
        formatted_sha256 = $formattedSha256
        format_idempotent = $idempotent
        rust_check = $checkState
        rust_check_exit = $check.ExitCode
        diagnostic_ids = $diagnosticIds
        diagnostic_sha256 = Get-Sha256Hex $diagnosticBytes
    }
    $entryIndex++
}

Remove-Item -LiteralPath $artifactRoot -Force

$manifest = [ordered]@{
    schema = 'sico.m22.corpus.v0'
    roots = @('semantic-cases', 'syntax-candidates', 'tests/end-to-end')
    entry_count = $entries.Count
    entries = $entries
}
$json = (ConvertTo-CanonicalJson $manifest) + "`n"
$output = Join-Path $root 'selfhost\corpus-v0.json'
$jsonBytes = [System.Text.UTF8Encoding]::new($false).GetBytes($json)
$buildManifest = [ordered]@{
    schema = 'sico.m22.script-build-corpus.v0'
    source_manifest_sha256 = Get-Sha256Hex $jsonBytes
    entry_count = $buildEntries.Count
    accepted_count = @($buildEntries | Where-Object rust_script_build -eq 'accepted').Count
    refused_count = @($buildEntries | Where-Object rust_script_build -eq 'refused').Count
    entries = $buildEntries
}
$buildJson = (ConvertTo-CanonicalJson $buildManifest) + "`n"
$buildOutput = Join-Path $root 'selfhost\script-build-corpus-v0.json'
if ($Verify) {
    # Serialization is deterministic (ConvertTo-CanonicalJson), so the frozen
    # bytes compare equal across Windows PowerShell 5.1 and PowerShell 7 hosts.
    $existing = [System.IO.File]::ReadAllText($output, [System.Text.Encoding]::UTF8)
    if ($existing -cne $json) {
        throw 'M22 corpus manifest is stale; run tools/update-m22-corpus.ps1'
    }
    $existingBuild = [System.IO.File]::ReadAllText($buildOutput, [System.Text.Encoding]::UTF8)
    if ($existingBuild -cne $buildJson) {
        throw 'M22 Script build manifest is stale; run tools/update-m22-corpus.ps1'
    }
    Write-Output "M22_CORPUS_CURRENT path=$output entries=$($entries.Count)"
    Write-Output "M22_SCRIPT_BUILD_CORPUS_CURRENT path=$buildOutput entries=$($buildEntries.Count)"
    exit 0
}
[System.IO.File]::WriteAllText($output, $json, [System.Text.UTF8Encoding]::new($false))
[System.IO.File]::WriteAllText($buildOutput, $buildJson, [System.Text.UTF8Encoding]::new($false))
Write-Output "M22_CORPUS_WRITTEN path=$output entries=$($entries.Count)"
Write-Output "M22_SCRIPT_BUILD_CORPUS_WRITTEN path=$buildOutput entries=$($buildEntries.Count)"
