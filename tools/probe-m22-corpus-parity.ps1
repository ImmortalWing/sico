<#
.SYNOPSIS
M22 W4 rejection-corpus parity probe (STEP-0561).

.DESCRIPTION
Checks the GUEST frontend's verdict against the RUST oracle's frozen rejection
verdict for the M22 frozen rejection corpus.  For every corpus entry the Rust
oracle refused (rust_check = "refused"), the guest self-host compiler must also
refuse the same source: the guest is run as `sico run selfhost/compiler.sico`
with the source on stdin, and the outcome is classified as:

  REFUSED <code>   the guest refused with a typed identity (ERR:...)
  LOWERED          the guest produced canonical IR (it accepted what Rust refused)
  FAILED <snippet> any other outcome (trap, fuel, unexpected output)

The parity verdict is `match` when the guest refuses (any ERR: code) and the
Rust also refused; `mismatch` when the guest lowered or failed.

The check records, per item, the path, the Rust diagnostic id set (frozen), the
guest's refusal code (or "lowered"/"failed"), and the parity verdict.  It writes
`selfhost/corpus-parity-v0.json` (frozen-format, deterministic serialization)
and prints a summary.  `-Limit` runs only the first N refusal entries (for a
fast development subset); `-Json` prints the machine-readable record.

Exit codes: 0 = all parity verdicts match, 1 = at least one mismatch.

Usage:
  pwsh -File tools/probe-m22-corpus-parity.ps1
  pwsh -File tools/probe-m22-corpus-parity.ps1 -Limit 12
  pwsh -File tools/probe-m22-corpus-parity.ps1 -Json
#>
[CmdletBinding()]
param(
    [int]$Limit = 0,
    [switch]$Json
)

# The guest writes its refusal identity to stderr; a native stderr line must
# be captured as text (2>&1 | Out-String) and not abort the loop, so the
# preference stays Continue (the same choice as probe-m22-frontend-frontier.ps1).
$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$cli = Join-Path $repoRoot 'target\debug\sico.exe'
$runner = Join-Path $repoRoot 'runner\sico-runner\target\debug\sico-runner.exe'
$corpusPath = Join-Path $repoRoot 'selfhost\corpus-v0.json'
$outputPath = Join-Path $repoRoot 'selfhost\corpus-parity-v0.json'

if (-not (Test-Path -LiteralPath $cli)) {
    Write-Error "sico CLI not found at $cli (build with: cargo build -p sico-cli)"
    exit 1
}
if (-not (Test-Path -LiteralPath $runner)) {
    Write-Error "sico-runner not found at $runner (build with: cargo build --manifest-path runner/sico-runner/Cargo.toml --bins)"
    exit 1
}
if (-not (Test-Path -LiteralPath $corpusPath)) {
    Write-Error "frozen corpus not found at $corpusPath (run tools/update-m22-corpus.ps1)"
    exit 1
}

$env:SICO_RUNNER = $runner

$corpus = Get-Content -LiteralPath $corpusPath -Raw | ConvertFrom-Json

# The refusal entries are the ones the Rust oracle refused at the check stage.
$refusals = @($corpus.entries | Where-Object { $_.rust_check -eq 'refused' })
if ($Limit -gt 0) {
    $refusals = @($refusals | Select-Object -First $Limit)
}

# Frozen-format serialization: deterministic, pretty-printed, key-order
# preserving, and array-preserving.  The result is assigned to $jsonText (not
# $json) because $json is case-insensitively the script's [switch]$Json
# parameter; assigning a string there would raise a String->SwitchParameter
# conversion error.
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
    if ($Value -is [int] -or $Value -is [long]) {
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

$records = @()
$mismatchCount = 0
foreach ($entry in $refusals) {
    $path = Join-Path $repoRoot $entry.path
    if (-not (Test-Path -LiteralPath $path)) {
        Write-Output ("MISSING path={0}" -f $entry.path)
        continue
    }
    $source = [System.IO.File]::ReadAllText($path)
    $source = $source.Replace("`r`n", "`n")
    $raw = ($source | & $cli run (Join-Path $repoRoot 'selfhost\compiler.sico') 2>&1 | Out-String)
    $flat = ($raw -replace "`r?`n", " ")
    $marker = $flat.IndexOf('ERR:')
    if ($marker -ge 0) {
        $code = $flat.Substring($marker)
        $end = $code.IndexOf('"')
        if ($end -gt 0) { $code = $code.Substring(0, $end) }
        $code = $code.TrimEnd('\', '"', ' ')
        $guestVerdict = "refused:$code"
        $parity = 'match'
    } elseif ($flat -match '"schema":"sico\.ir\.v0"') {
        $guestVerdict = 'lowered'
        $parity = 'mismatch'
    } else {
        $guestVerdict = 'failed:' + $flat.Substring(0, [Math]::Min(80, $flat.Length))
        $parity = 'mismatch'
    }
    if ($parity -eq 'mismatch') { $mismatchCount++ }
    $records += [ordered]@{
        path = $entry.path
        rust_diagnostic_ids = @($entry.diagnostic_ids)
        guest_verdict = $guestVerdict
        parity = $parity
    }
}

$manifest = [ordered]@{
    schema = 'sico.m22.corpus-parity.v0'
    checked_count = $records.Count
    mismatch_count = $mismatchCount
    entries = $records
}
$jsonText = (ConvertTo-CanonicalJson $manifest) + "`n"

if (-not $Json) {
    [System.IO.File]::WriteAllText($outputPath, $jsonText, [System.Text.UTF8Encoding]::new($false))
}

foreach ($record in $records) {
    $status = if ($record.parity -eq 'match') { 'OK  ' } else { 'MISMATCH' }
    Write-Output ("{0} {1,-58} rust={2,-10} guest={3}" -f $status, $record.path, ($record.rust_diagnostic_ids -join ','), $record.guest_verdict)
}
Write-Output ("CORPUS_PARITY checked={0} mismatch={1}" -f $records.Count, $mismatchCount)
if ($Json) {
    Write-Output $jsonText
}
if ($mismatchCount -eq 0) {
    exit 0
}
exit 1
