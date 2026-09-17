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
    $start.StandardInputEncoding = [System.Text.UTF8Encoding]::new($false)
    $start.StandardOutputEncoding = [System.Text.UTF8Encoding]::new($false)
    $start.StandardErrorEncoding = [System.Text.UTF8Encoding]::new($false)
    foreach ($argument in $Arguments) {
        [void]$start.ArgumentList.Add($argument)
    }
    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $start
    [void]$process.Start()
    if ($null -ne $StdinText) {
        $process.StandardInput.Write($StdinText)
    }
    $process.StandardInput.Close()
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
    $digest = [System.Security.Cryptography.SHA256]::HashData($Bytes)
    return [Convert]::ToHexString($digest).ToLowerInvariant()
}

$relativePaths = @(
    & rg --files syntax-candidates semantic-cases tests/end-to-end -g '*.sico'
) | ForEach-Object { $_.Replace('\', '/') } | Sort-Object -CaseSensitive

$artifactRoot = Join-Path ([System.IO.Path]::GetTempPath()) ("sico-m22-corpus-{0}" -f [Guid]::NewGuid().ToString('N'))
[void](New-Item -ItemType Directory -Path $artifactRoot)

$entryIndex = 0
$buildEntries = @()
$entries = foreach ($relativePath in $relativePaths) {
    $nativePath = Join-Path $root $relativePath
    $bytes = [System.IO.File]::ReadAllBytes($nativePath)
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
        if ($diagnosticText.Contains('lexical error', [StringComparison]::OrdinalIgnoreCase)) {
            $diagnosticIds = @('LEXICAL')
        } elseif ($diagnosticText.Contains('syntax', [StringComparison]::OrdinalIgnoreCase)) {
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
        $componentReproducible = [System.Linq.Enumerable]::SequenceEqual[byte]($artifactBytes, $reproBytes)
        if (-not $componentReproducible) {
            throw "script-v0 artifact is not reproducible: $relativePath"
        }
        Remove-Item -LiteralPath $artifactPath -Force
        Remove-Item -LiteralPath $reproPath -Force
    } elseif (Test-Path -LiteralPath $artifactPath) {
        throw "sico build refusal left an artifact: $relativePath"
    }
    $buildDiagnosticText = "$($build.Stdout)`n--stderr--`n$($build.Stderr)"
    $buildDiagnosticText = $buildDiagnosticText.Replace($artifactPath, '<artifact>', [StringComparison]::Ordinal)
    $buildDiagnosticBytes = [System.Text.UTF8Encoding]::new($false).GetBytes($buildDiagnosticText)
    $buildDiagnosticIds = @(
        [regex]::Matches($buildDiagnosticText, '\bE\d{4}\b') |
            ForEach-Object { $_.Value } |
            Sort-Object -Unique
    )
    if ($buildDiagnosticIds.Count -eq 0 -and $build.ExitCode -ne 0) {
        if ($buildDiagnosticText.Contains('lexical error', [StringComparison]::OrdinalIgnoreCase)) {
            $buildDiagnosticIds = @('LEXICAL')
        } elseif ($buildDiagnosticText.Contains('script profile requires', [StringComparison]::OrdinalIgnoreCase)) {
            $buildDiagnosticIds = @('SCRIPT_ABI')
        } elseif ($buildDiagnosticText.Contains('syntax error', [StringComparison]::OrdinalIgnoreCase)) {
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
$json = ($manifest | ConvertTo-Json -Depth 8) + "`n"
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
$buildJson = ($buildManifest | ConvertTo-Json -Depth 8) + "`n"
$buildOutput = Join-Path $root 'selfhost\script-build-corpus-v0.json'
if ($Verify) {
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
