# M23 kickoff ceremony census (execution card 23-A / M23 plan §7-8).
# Pure text measurement over two corpora; zero grammar or source changes.
# Schema: sico.m23.ceremony.v0
#
# Corpus A (self-host): the 8 tracked selfhost/*.sico sources (the
# STEP-0278 denominator; the historical eleven-file snapshot is retired).
# Corpus B (application profile): tests/end-to-end/*.sico.
#
# Per-field pattern definitions (byte-deterministic; no timestamps):
#   arith_checked_match_blocks   regex 'match\s+(I64|U64)\.checked_(add|sub|mul)\('
#   comparison_if_lines          '^\s*if\s+(I64|U64|F64|Text|Bool)\.(equal|not_equal|less_than|less_equal|greater_than|greater_equal)\('
#   comparison_return_lines      '^\s*return\s+(I64|U64|F64|Text|Bool)\.(equal|not_equal|less_than|less_equal|greater_than|greater_equal)\('
#   nested_comparison_if2_sites  comparison_if_lines whose next non-blank line
#                                is also a comparison_if_line at deeper indent
#   bind_value_match_blocks      match blocks (bracketed by the match line and
#                                the same-indent 'end match') whose body sets
#                                exactly one target via 'set <name> ='; the
#                                ok/error + set + join-read conditional-bind
#                                proxy of plan 8.2
#   literal_constructor_calls    '\b(I64|U64|F64|Bool)\.literal\('
#   chars_api_call_sites         'sico\.text\.(length|char_at)\('
#   word_kind_chars_call_sites   the same calls inside the formatter.sico
#                                'word_kind' function region (plan 8.4 corpus)
# All counts are line/textual proxies in the census-ec4-records.ps1 pattern;
# each JSON field names its exact pattern so an independent run recomputes it.

param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$predRe = '(I64|U64|F64|Text|Bool)\.(equal|not_equal|less_than|less_equal|greater_than|greater_equal)\('

function Measure-File {
    param([string]$Path)

    $lines = [IO.File]::ReadAllLines($Path)
    $text = [IO.File]::ReadAllText($Path)

    $m = [ordered]@{
        file = $Path.Substring($RepoRoot.Length + 1).Replace('\', '/')
        total_lines = $lines.Count
        bytes = (Get-Item -LiteralPath $Path).Length
    }

    $m.arith_checked_match_blocks = [regex]::Matches($text, 'match\s+(I64|U64)\.checked_(add|sub|mul)\(').Count
    $m.comparison_if_lines = [regex]::Matches($text, "(?m)^\s*if\s+$predRe").Count
    $m.comparison_return_lines = [regex]::Matches($text, "(?m)^\s*return\s+$predRe").Count

    $nested = 0
    for ($i = 0; $i -lt $lines.Count - 1; $i++) {
        if ($lines[$i] -notmatch "(?m)^\s*if\s+$predRe") { continue }
        for ($j = $i + 1; $j -lt $lines.Count; $j++) {
            $ln = $lines[$j]
            if ($ln.Trim().Length -eq 0) { continue }
            if ($ln -match "(?m)^\s*if\s+$predRe" -and
                ($ln.Length - $ln.TrimStart().Length) -gt ($lines[$i].Length - $lines[$i].TrimStart().Length)) {
                $nested++
            }
            break
        }
    }
    $m.nested_comparison_if2_sites = $nested

    $bindBlocks = 0
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($lines[$i] -notmatch '(?m)^(\s*)match\s') { continue }
        $indent = $lines[$i].Length - $lines[$i].TrimStart().Length
        $targets = @{}
        $isBind = $false
        for ($j = $i + 1; $j -lt $lines.Count; $j++) {
            $ln = $lines[$j]
            $ind = $ln.Length - $ln.TrimStart().Length
            if ($ln.Trim() -eq 'end match' -and $ind -eq $indent) { break }
            if ($ln -match '^\s*set\s+([A-Za-z_][A-Za-z0-9_]*)\s*=') {
                $isBind = $true
                $targets[$Matches[1]] = $true
            }
        }
        if ($isBind -and $targets.Count -eq 1) { $bindBlocks++ }
    }
    $m.bind_value_match_blocks = $bindBlocks

    $m.literal_constructor_calls = [regex]::Matches($text, '\b(I64|U64|F64|Bool)\.literal\(').Count
    $m.chars_api_call_sites = [regex]::Matches($text, 'sico\.text\.(length|char_at)\(').Count

    $m.word_kind_chars_call_sites = 0
    if ((Split-Path -Leaf $Path) -eq 'formatter.sico') {
        $wm = [regex]::Match($text, '(?ms)^function word_kind\b.*?^end function\s*$')
        if ($wm.Success) {
            $m.word_kind_chars_call_sites = [regex]::Matches($wm.Value, 'sico\.text\.(length|char_at)\(').Count
        }
    }

    return $m
}

$selfhostFiles = Get-ChildItem -LiteralPath (Join-Path $RepoRoot 'selfhost') -Filter '*.sico' | Sort-Object Name
$e2eFiles = Get-ChildItem -LiteralPath (Join-Path $RepoRoot 'tests\end-to-end') -Filter '*.sico' | Sort-Object Name

$perFile = @()
foreach ($f in $selfhostFiles) { $perFile += (Measure-File -Path $f.FullName) }

$appPerFile = @()
$appTotals = $null
if ($e2eFiles.Count -gt 0) {
    foreach ($f in $e2eFiles) { $appPerFile += (Measure-File -Path $f.FullName) }
}

$countFields = @('total_lines','bytes','arith_checked_match_blocks','comparison_if_lines',
    'comparison_return_lines','nested_comparison_if2_sites','bind_value_match_blocks',
    'literal_constructor_calls','chars_api_call_sites','word_kind_chars_call_sites')

function Sum-Field {
    param([object[]]$Rows, [string]$Field)
    $sum = 0
    foreach ($r in $Rows) { $sum += [int64]$r[$Field] }
    return $sum
}

function Totals {
    param([object[]]$Rows)
    $t = [ordered]@{}
    foreach ($f in $countFields) { $t[$f] = (Sum-Field $Rows $f) }
    $t.files = $Rows.Count
    $t.literal_calls_per_kb = [math]::Round((Sum-Field $Rows 'literal_constructor_calls') /
        [Math]::Max(1.0, (Sum-Field $Rows 'bytes') / 1024.0), 3)
    $t.comparison_lines_per_1000 = [math]::Round((Sum-Field $Rows 'comparison_if_lines') /
        [Math]::Max(1.0, (Sum-Field $Rows 'total_lines') / 1000.0), 3)
    return $t
}

$selfhostTotals = Totals $perFile

$appTotalsOut = [ordered]@{ files = 0 }
if ($appPerFile.Count -gt 0) { $appTotalsOut = Totals $appPerFile }

$result = [ordered]@{
    schema = 'sico.m23.ceremony.v0'
    step = 'STEP-0284'
    card = '23-A'
    repo_root = $RepoRoot
    pattern_notes = 'textual line/regex proxies per file header; recomputable byte-deterministically'
    gaps = [ordered]@{
        ai_generated_corpus = 'not measured; owner-gated external corpus (plan 8.3) recorded as gap'
        runtime_call_or_alloc_counts = 'not measured; runner CLI exposes no call/allocation counters (plan 8.4 runtime side recorded as gap)'
    }
    selfhost = [ordered]@{ totals = $selfhostTotals; per_file = $perFile }
    application = [ordered]@{ totals = $appTotalsOut; per_file_count = $appPerFile.Count }
    ok = 'CENSUS_M23_CEREMONY_OK'
}

$out = [IO.Path]::Combine($RepoRoot, 'docs', 'reports', 'm23-ceremony-census-2026-09-25.json')
[IO.File]::WriteAllText($out, ($result | ConvertTo-Json -Depth 6) + "`n", [Text.UTF8Encoding]::new($false))
Write-Host "CENSUS_M23_CEREMONY_OK selfhost_files=$($perFile.Count) app_files=$($appPerFile.Count)"
Write-Host "selfhost: lines=$($selfhostTotals.total_lines) checked_matches=$($selfhostTotals.arith_checked_match_blocks) cmp_if=$($selfhostTotals.comparison_if_lines) nested_if2=$($selfhostTotals.nested_comparison_if2_sites) bind_matches=$($selfhostTotals.bind_value_match_blocks) literals=$($selfhostTotals.literal_constructor_calls) chars_calls=$($selfhostTotals.chars_api_call_sites)"
