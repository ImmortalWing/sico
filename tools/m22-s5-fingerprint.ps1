<#
.SYNOPSIS
M22 S5 dev-battery fingerprint (STEP-0565).

.DESCRIPTION
Computes a deterministic sha256 fingerprint of the source tree so the
development-tier battery (`tools/validate-m22-s5-dev.ps1`) can tell whether a
saved per-step state still describes the current tree.

Scope (AGENTS.md 10.1.2 / 10.11 A): every file under the repository root is
hashed, EXCEPT the three load-bearing exclusions

  * any path under a `target` directory  (build output; the battery itself
    writes its logs and state here, so without this exclusion every run would
    invalidate its own previous green = the STEP-0545 6.1 self-pollution);
  * any `__pycache__` directory          (Python bytecode cache);
  * any `*.pyc` file.

The fingerprint covers untracked files too: the battery reads untracked
fixtures and scripts, so a fingerprint built from `git ls-files` alone would
miss exactly the working-copy changes the gate exists to catch. `target`
covers the gitignored build output; everything else in the tree counts.

Determinism: entries are sorted by relative path (OrdinalIgnoreCase), each
entry is hashed as `relpath\tn\t<sha256(file bytes)>`, and the fingerprint is
`sha256(fileCount\n + concat(entries))`. Same tree => same fingerprint.

Usage:
  powershell -ExecutionPolicy Bypass -File tools/m22-s5-fingerprint.ps1
  powershell -ExecutionPolicy Bypass -File tools/m22-s5-fingerprint.ps1 -AsJson
  powershell -ExecutionPolicy Bypass -File tools/m22-s5-fingerprint.ps1 -SaveFile <path>
  powershell -ExecutionPolicy Bypass -File tools/m22-s5-fingerprint.ps1 -TestFile <path>

Exit codes: 0 fingerprint computed / matches; 1 tool or repository error;
2 fingerprint mismatch (-TestFile); 3 saved fingerprint file missing.
#>
[CmdletBinding()]
param(
    [switch]$AsJson,
    [string]$SaveFile,
    [string]$TestFile,
    [switch]$SelfTest
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent $PSScriptRoot
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$sha = [System.Security.Cryptography.SHA256]::Create()
$utf8 = New-Object System.Text.UTF8Encoding($false)

function Get-TreeFingerprint([string]$root) {
    $fullRoot = (Resolve-Path -LiteralPath $root).Path
    $rootWithSep = $fullRoot.TrimEnd('\') + '\'
    $items = Get-ChildItem -LiteralPath $fullRoot -Recurse -Force -File `
        -Attributes !ReparsePoint -ErrorAction SilentlyContinue

    $entries = New-Object System.Collections.Generic.List[string]
    foreach ($item in $items) {
        $rel = $item.FullName.Substring($rootWithSep.Length).Replace('\', '/')
        $segments = $rel.Split('/')
        $excluded = $false
        for ($i = 0; $i -lt $segments.Length - 1; $i++) {
            if ($segments[$i] -eq 'target' -or $segments[$i] -eq '__pycache__') { $excluded = $true; break }
        }
        if (-not $excluded -and $rel.EndsWith('.pyc', [StringComparison]::OrdinalIgnoreCase)) { $excluded = $true }
        if ($excluded) { continue }

        $bytes = [System.IO.File]::ReadAllBytes($item.FullName)
        $fileHash = [BitConverter]::ToString($sha.ComputeHash($bytes)).Replace('-', '').ToLowerInvariant()
        $entries.Add(("{0}`t{1}`t{2}" -f $rel, $item.Length, $fileHash))
    }

    $ordered = $entries.ToArray()
    [Array]::Sort($ordered, [System.StringComparer]::OrdinalIgnoreCase)
    $material = [System.Text.StringBuilder]::new()
    [void]$material.Append($ordered.Count).Append("`n")
    foreach ($line in $ordered) { [void]$material.Append($line).Append("`n") }
    $digest = [BitConverter]::ToString($sha.ComputeHash($utf8.GetBytes($material.ToString()))).Replace('-', '').ToLowerInvariant()
    [pscustomobject]@{
        Fingerprint = $digest
        FileCount   = $ordered.Count
        Root        = $fullRoot
    }
}

if ($SelfTest) {
    $probe = Join-Path $repoRoot 'target\tmp\m22-s5-fingerprint-selftest'
    New-Item -ItemType Directory -Force -Path $probe | Out-Null
    $payload = Join-Path $probe 'payload.txt'
    [System.IO.File]::WriteAllText($payload, "fingerprint self test`n", $utf8)
    $first = Get-TreeFingerprint $probe
    $second = Get-TreeFingerprint $probe
    [System.IO.File]::WriteAllText($payload, "fingerprint self test changed`n", $utf8)
    $third = Get-TreeFingerprint $probe
    $pycDir = Join-Path $probe '__pycache__'
    New-Item -ItemType Directory -Force -Path $pycDir | Out-Null
    [System.IO.File]::WriteAllBytes((Join-Path $pycDir 'module.pyc'), @(0x01, 0x02, 0x03))
    $fourth = Get-TreeFingerprint $probe
    Remove-Item -Recurse -Force -Path $probe
    if ($first.Fingerprint -ne $second.Fingerprint) { Write-Error 'self-test: identical tree produced different fingerprints'; exit 1 }
    if ($first.Fingerprint -eq $third.Fingerprint) { Write-Error 'self-test: changed content did not change the fingerprint'; exit 1 }
    if ($third.Fingerprint -ne $fourth.Fingerprint) { Write-Error 'self-test: __pycache__ was not excluded'; exit 1 }
    Write-Host ("FINGERPRINT_SELF_TEST_GREEN files={0} fp={1}" -f $first.FileCount, $first.Fingerprint)
    exit 0
}

if (-not (Test-Path -LiteralPath (Join-Path $repoRoot '.git'))) {
    Write-Error ("not a git work tree: {0}" -f $repoRoot)
    exit 1
}

$fingerprint = Get-TreeFingerprint $repoRoot

if ($SaveFile) {
    $payload = [pscustomobject]@{
        schema      = 'sico.m22.s5.fingerprint.v1'
        fingerprint = $fingerprint.Fingerprint
        fileCount   = $fingerprint.FileCount
        savedAtUtc  = [DateTime]::UtcNow.ToString('o')
    }
    $json = $payload | ConvertTo-Json
    $directory = Split-Path -Parent $SaveFile
    if ($directory) { New-Item -ItemType Directory -Force -Path $directory | Out-Null }
    [System.IO.File]::WriteAllText($SaveFile, $json + "`n", $utf8)
}

if ($AsJson) {
    $fingerprint | ConvertTo-Json -Compress | Write-Host
    exit 0
}

if ($TestFile) {
    if (-not (Test-Path -LiteralPath $TestFile -PathType Leaf)) {
        Write-Host ("FINGERPRINT_NO_SAVED_STATE file={0}" -f $TestFile)
        exit 3
    }
    $saved = Get-Content -LiteralPath $TestFile -Raw | ConvertFrom-Json
    if ($saved.fingerprint -ne $fingerprint.Fingerprint) {
        Write-Host ("FINGERPRINT_MISMATCH saved={0} current={1} files={2}" -f $saved.fingerprint, $fingerprint.Fingerprint, $fingerprint.FileCount)
        exit 2
    }
    Write-Host ("FINGERPRINT_MATCH {0} files={1}" -f $fingerprint.Fingerprint, $fingerprint.FileCount)
    exit 0
}

Write-Host ("FINGERPRINT {0} files={1} root={2}" -f $fingerprint.Fingerprint, $fingerprint.FileCount, $fingerprint.Root)
exit 0
