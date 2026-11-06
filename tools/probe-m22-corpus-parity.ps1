<#
.SYNOPSIS
M22 W4 rejection-corpus parity probe with diagnostic-id mapping contract (STEP-0564).

.DESCRIPTION
Extends the STEP-0561 verdict-parity probe.  For every frozen corpus entry the
Rust oracle refused at the check stage (rust_check = "refused"), the guest
self-host compiler is run (`sico run selfhost/compiler.sico`, source on stdin)
and its refusal identity is compared against the Rust refusal.

Per entry the probe records:
  guest_verdict   REFUSED:<ERR:E-SH-IR-*> | LOWERED | FAILED:<snippet>
  rust_verdict    REFUSED:<frozen diagnostic id>   (from selfhost/corpus-v0.json)
  mapped_id       the registered guest equivalence class, or "unmapped"
  parity          exact | structural | zero

Parity levels (mapping contract in selfhost/corpus-parity-map-v1.json):
  exact       guest refused AND the (rust id -> guest code) pair is registered
  structural  guest refused but the pair is NOT registered (verdict-only parity)
  zero        guest did NOT refuse (lowered = soundness hole, failed = fuel wall)

SPAN AXIS: the Rust oracle emits full spans (`check --json`), but the frozen
corpus diagnostic ids carry no span and the guest emits a bare ERR:E-SH-IR-*
code with no span.  True (verdict, id, span) triple parity is therefore NOT
reachable; the probe reports the span axis as a residual gap and does not
assert span equality.  `-RustSpans` additionally re-runs `sico check --json`
per entry to capture the live Rust span as corroborating evidence only (it is
not compared, because there is no guest span to compare against).

The probe writes its per-entry parity report to `selfhost/corpus-parity-v0.json`
(extended with mapped_id / parity level / rust_verdict; deterministic
serialization) and reads the mapping contract from
`selfhost/corpus-parity-map-v1.json`.  It prints a summary.  `-Limit N` runs
only the first N refusal entries.
Exit codes: 0 = no `zero` parity (no soundness hole / no new fuel wall beyond
the 3 registered ones), 1 = at least one `zero` parity.

Usage:
  powershell -NoProfile -File tools/probe-m22-corpus-parity.ps1
  powershell -NoProfile -File tools/probe-m22-corpus-parity.ps1 -Limit 12
  powershell -NoProfile -File tools/probe-m22-corpus-parity.ps1 -Json
  powershell -NoProfile -File tools/probe-m22-corpus-parity.ps1 -RustSpans -Limit 4
#>
[CmdletBinding()]
param(
    [int]$Limit = 0,
    [switch]$Json,
    [switch]$RustSpans
)

# The guest writes its refusal identity to stderr; a native stderr line must be
# captured as text (2>&1 | Out-String) and not abort the loop, so the preference
# stays Continue (same choice as probe-m22-frontend-frontier.ps1).
$ErrorActionPreference = 'Continue'
Set-StrictMode -Version Latest

$repoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$cli = Join-Path $repoRoot 'target\debug\sico.exe'
$runner = Join-Path $repoRoot 'runner\sico-runner\target\debug\sico-runner.exe'
$corpusPath = Join-Path $repoRoot 'selfhost\corpus-v0.json'
$mapPath = Join-Path $repoRoot 'selfhost\corpus-parity-map-v1.json'
$outputPath = Join-Path $repoRoot 'selfhost\corpus-parity-v0.json'

foreach ($required in @($cli, $runner, $corpusPath, $mapPath)) {
    if (-not (Test-Path -LiteralPath $required)) {
        Write-Error "required input not found: $required"
        exit 1
    }
}

$env:SICO_RUNNER = $runner

$corpus = Get-Content -LiteralPath $corpusPath -Raw | ConvertFrom-Json
$mapRaw = Get-Content -LiteralPath $mapPath -Raw | ConvertFrom-Json

# --- mapping-artifact validation (typed rejection, no silent approximation) ---
if ($mapRaw.schema -ne 'sico.m22.corpus-parity-map.v1') {
    Write-Error "unexpected mapping schema '$($mapRaw.schema)' (expected sico.m22.corpus-parity-map.v1)"
    exit 1
}
if ([int]$mapRaw.version -ne 1) {
    Write-Error "unsupported mapping version $($mapRaw.version) (expected 1)"
    exit 1
}
$mapEntries = @($mapRaw.mappings)
if ($mapEntries.Count -eq 0) {
    Write-Error "mapping artifact registers no equivalence classes"
    exit 1
}
# rust id -> set of registered guest codes
$rustToGuest = @{}
$pairSeen = @{}
foreach ($m in $mapEntries) {
    if ([string]::IsNullOrWhiteSpace($m.rust_code) -or [string]::IsNullOrWhiteSpace($m.guest_code)) {
        Write-Error "mapping entry has empty rust_code/guest_code"
        exit 1
    }
    if ($m.grain -ne 'refined' -and $m.grain -ne 'coarse') {
        Write-Error "mapping grain '$($m.grain)' is not refined|coarse"
        exit 1
    }
    $pairKey = "$($m.rust_code)|$($m.guest_code)"
    if ($pairSeen.ContainsKey($pairKey)) {
        Write-Error "duplicate mapping pair $pairKey"
        exit 1
    }
    $pairSeen[$pairKey] = $true
    if (-not $rustToGuest.ContainsKey($m.rust_code)) {
        $rustToGuest[$m.rust_code] = @{}
    }
    $rustToGuest[$m.rust_code][$m.guest_code] = $true
}

# The refusal entries are the ones the Rust oracle refused at the check stage.
$refusals = @($corpus.entries | Where-Object { $_.rust_check -eq 'refused' })
if ($Limit -gt 0) {
    $refusals = @($refusals | Select-Object -First $Limit)
}

# Frozen-format serialization: deterministic, pretty-printed, key-order
# preserving, and array-preserving.  Assigned to $jsonText because $json is
# case-insensitively this script's [switch]$Json parameter.
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

# Capture the live Rust span for one entry via `sico check --json`.  Returns a
# compact evidence object; this is corroboration only (no guest span exists to
# compare against), so any failure here degrades to span_evidence="unavailable"
# rather than failing the whole probe.
function Get-RustSpanEvidence {
    param([string]$RelativePath)
    $json = Join-Path $env:TEMP ("sico-span-" + [System.Guid]::NewGuid().ToString('N') + ".json")
    try {
        $full = Join-Path $repoRoot $RelativePath
        $out = & $cli check --json $full 2>$null
        $exit = $LASTEXITCODE
        [System.IO.File]::WriteAllText($json, ($out -join "`n"), [System.Text.UTF8Encoding]::new($false))
        if ($exit -eq 0) { return @{ span_evidence = 'rust-accepted-no-span' } }
        $parsed = Get-Content -LiteralPath $json -Raw | ConvertFrom-Json
        $diags = @($parsed.diagnostics)
        if ($diags.Count -eq 0) { return @{ span_evidence = 'rust-refused-no-diagnostic' } }
        $first = $diags[0]
        return [ordered]@{
            span_evidence = 'rust-only'
            rust_code = $first.code
            rust_key = $first.key
            rust_span = ("{0}:{1}:{2}" -f $first.range.start.line, $first.range.start.column, $first.range.start.byte)
            rust_diagnostic_count = $diags.Count
            note = 'live `sico check --json`; guest emits no span, so no span comparison is performed'
        }
    } catch {
        return @{ span_evidence = 'unavailable' }
    } finally {
        if (Test-Path -LiteralPath $json) { Remove-Item -LiteralPath $json -Force -ErrorAction SilentlyContinue }
    }
}

$records = @()
$exactCount = 0
$structuralCount = 0
$zeroCount = 0
$unmappedRustIds = @{}
foreach ($entry in $refusals) {
    $path = Join-Path $repoRoot $entry.path
    if (-not (Test-Path -LiteralPath $path)) {
        Write-Output ("MISSING path={0}" -f $entry.path)
        continue
    }
    $rustIds = @($entry.diagnostic_ids)
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
        $guestVerdict = "REFUSED:$code"

        # mapping lookup: does any registered class for these rust ids emit this guest code?
        $mapped = 'unmapped'
        foreach ($rid in $rustIds) {
            if ($rustToGuest.ContainsKey($rid) -and $rustToGuest[$rid].ContainsKey($code)) {
                $mapped = $code
                break
            }
        }
        if ($mapped -eq 'unmapped') {
            foreach ($rid in $rustIds) { $unmappedRustIds[$rid] = $true }
            $parity = 'structural'
            $structuralCount++
        } else {
            $parity = 'exact'
            $exactCount++
        }
    } elseif ($flat -match '"schema":"sico\.ir\.v0"') {
        $guestVerdict = 'LOWERED'
        $parity = 'zero'
        $zeroCount++
    } else {
        $guestVerdict = 'FAILED:' + $flat.Substring(0, [Math]::Min(80, $flat.Length))
        $parity = 'zero'
        $zeroCount++
    }

    $record = [ordered]@{
        path = $entry.path
        rust_verdict = "REFUSED:" + ($rustIds -join ',')
        guest_verdict = $guestVerdict
        mapped_id = $mapped
        parity = $parity
    }
    if ($RustSpans) {
        $record.span = Get-RustSpanEvidence -RelativePath $entry.path
    }
    $records += $record
}

$manifest = [ordered]@{
    schema = 'sico.m22.corpus-parity.v0'
    mapped_by = [ordered]@{
        schema = $mapRaw.schema
        version = [int]$mapRaw.version
    }
    checked_count = $records.Count
    exact_parity_count = $exactCount
    structural_parity_count = $structuralCount
    zero_parity_count = $zeroCount
    mapped_id_coverage = [ordered]@{
        registered_classes = $mapEntries.Count
        refined = @($mapEntries | Where-Object { $_.grain -eq 'refined' }).Count
        coarse = @($mapEntries | Where-Object { $_.grain -eq 'coarse' }).Count
    }
    span_axis = 'not-compared: guest emits no span (see selfhost/corpus-parity-map-v1.json .span_axis)'
    unmapped_rust_ids = @($unmappedRustIds.Keys | Sort-Object)
    entries = $records
}
$jsonText = (ConvertTo-CanonicalJson $manifest) + "`n"

if (-not $Json) {
    [System.IO.File]::WriteAllText($outputPath, $jsonText, [System.Text.UTF8Encoding]::new($false))
}

foreach ($record in $records) {
    $status = switch ($record.parity) {
        'exact' { 'EXACT   ' }
        'structural' { 'STRUCT  ' }
        default { 'ZERO    ' }
    }
    Write-Output ("{0} {1,-58} rust={2,-16} guest={3,-34} mapped={4}" -f `
        $status, $record.path, $record.rust_verdict, $record.guest_verdict, $record.mapped_id)
}
Write-Output ("CORPUS_PARITY checked={0} exact={1} structural={2} zero={3}" -f `
    $records.Count, $exactCount, $structuralCount, $zeroCount)
Write-Output ("MAPPING registered_classes={0} (refined={1} coarse={2}) unmapped_rust_ids=[{3}]" -f `
    $mapEntries.Count, $manifest.mapped_id_coverage.refined, $manifest.mapped_id_coverage.coarse, `
    (($manifest.unmapped_rust_ids) -join ','))
if ($Json) {
    Write-Output $jsonText
}
if ($zeroCount -eq 0) {
    exit 0
}
exit 1