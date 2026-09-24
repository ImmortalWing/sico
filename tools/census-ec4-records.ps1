# EC-4 census (STEP-0270 / RFC-0047 acceptance freeze).
# Pure text measurement over two corpora; zero grammar or source changes.
# Schema: sico.m22.ec4.v0
#
# Corpus A (self-host): list-ceremony line share is the proxy ceiling for how
# much SOA table-threading authority RFC-0047 records can retire.
# Corpus B (application profile, M14 e2e): map-as-struct points and wide
# signatures are the measured demand for nominal records in application code.

param(
    [string]$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot '..')).Path
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Get-ParamCount {
    param([string]$Sig)
    # count top-level commas at paren depth 0 (signature text excludes outer parens)
    $depth = 0; $commas = 0
    foreach ($ch in $Sig.ToCharArray()) {
        switch ($ch) {
            '(' { $depth++ }
            ')' { if ($depth -gt 0) { $depth-- } }
            ',' { if ($depth -eq 0) { $commas++ } }
        }
    }
    return $commas + 1
}

function Measure-File {
    param([string]$Path, [string]$Kind)

    $lines = Get-Content -LiteralPath $Path
    $text = $lines -join "`n"

    $m = [ordered]@{
        file = (Split-Path -Leaf $Path)
        kind = $Kind
        total_lines = $lines.Count
    }

    if ($Kind -eq 'selfhost') {
        $ceremony = 0
        foreach ($ln in $lines) {
            if ($ln -match 'sico\.list\.(init|append|get|length)') { $ceremony++ }
        }
        $m.list_ceremony_lines = $ceremony
        $m.list_ceremony_line_share = [math]::Round($ceremony / [math]::Max(1, $lines.Count), 6)

        $fnMatches = [regex]::Matches($text, 'function\s+\w+\s*\((.*?)\)\s*returns', 'Singleline')
        $wide = 0; $listParamsTotal = 0
        foreach ($f in $fnMatches) {
            $sig = $f.Groups[1].Value
            $listParams = [regex]::Matches($sig, 'List\[').Count
            $listParamsTotal += $listParams
            if ($listParams -ge 3) { $wide++ }
        }
        $m.function_count = $fnMatches.Count
        $m.functions_with_ge3_list_params = $wide
        $m.list_param_occurrences = $listParamsTotal
    }
    else {
        $putMatches = [regex]::Matches($text, 'sico\.map\.put\[[^\]]+\]\s*\(')
        $literalKeys = [regex]::Matches($text, 'sico\.map\.put\[[^\]]+\]\s*\((?s:.*?)\,\s*"([^"]+)"\s*\,')
        $uniqueKeys = @{}
        foreach ($k in $literalKeys) { $uniqueKeys[$k.Groups[1].Value] = $true }
        $typedText = [regex]::Matches($text, '\bmap\.(put|empty|get|contains)\[Text\s*,').Count

        $fnMatches = [regex]::Matches($text, 'function\s+\w+\s*\((.*?)\)\s*returns', 'Singleline')
        $wide = 0
        foreach ($f in $fnMatches) {
            if ((Get-ParamCount $f.Groups[1].Value) -ge 5) { $wide++ }
        }

        $m.map_put_call_sites = $putMatches.Count
        $m.map_put_literal_key_sites = $literalKeys.Count
        $m.map_put_distinct_literal_keys = $uniqueKeys.Count
        $m.map_text_typed_ops = $typedText
        $m.function_count = $fnMatches.Count
        $m.functions_with_ge5_params = $wide
    }
    return $m
}

$selfhostFiles = Get-ChildItem -LiteralPath (Join-Path $RepoRoot 'selfhost') -Filter '*.sico' | Sort-Object Name
$e2eFiles = Get-ChildItem -LiteralPath (Join-Path $RepoRoot 'tests\end-to-end') -Filter '*.sico' | Sort-Object Name

$perFile = @()
foreach ($f in $selfhostFiles) { $perFile += (Measure-File -Path $f.FullName -Kind 'selfhost') }
foreach ($f in $e2eFiles) { $perFile += (Measure-File -Path $f.FullName -Kind 'application') }

function Sum-Field {
    param([object[]]$Rows, [string]$Field)
    $sum = 0
    foreach ($r in $Rows) { $sum += [int64]$r[$Field] }
    return $sum
}

$sh = @($perFile | Where-Object { $_.kind -eq 'selfhost' })
$ap = @($perFile | Where-Object { $_.kind -eq 'application' })

$totals = [ordered]@{
    selfhost = [ordered]@{
        files = $sh.Count
        total_lines = (Sum-Field $sh 'total_lines')
        list_ceremony_lines = (Sum-Field $sh 'list_ceremony_lines')
        list_ceremony_line_share = [math]::Round((Sum-Field $sh 'list_ceremony_lines') / [math]::Max(1, (Sum-Field $sh 'total_lines')), 6)
        function_count = (Sum-Field $sh 'function_count')
        functions_with_ge3_list_params = (Sum-Field $sh 'functions_with_ge3_list_params')
        list_param_occurrences = (Sum-Field $sh 'list_param_occurrences')
    }
    application = [ordered]@{
        files = $ap.Count
        total_lines = (Sum-Field $ap 'total_lines')
        map_put_call_sites = (Sum-Field $ap 'map_put_call_sites')
        map_put_literal_key_sites = (Sum-Field $ap 'map_put_literal_key_sites')
        map_put_distinct_literal_keys = (Sum-Field $ap 'map_put_distinct_literal_keys')
        map_text_typed_ops = (Sum-Field $ap 'map_text_typed_ops')
        function_count = (Sum-Field $ap 'function_count')
        functions_with_ge5_params = (Sum-Field $ap 'functions_with_ge5_params')
    }
}

$result = [ordered]@{
    schema = 'sico.m22.ec4.v0'
    generated_at = (Get-Date).ToString('yyyy-MM-dd HH:mm:ss')
    step = 'STEP-0270'
    rfc = 'RFC-0047'
    repo_root = $RepoRoot
    totals = $totals
    per_file = $perFile
}

$out = $result | ConvertTo-Json -Depth 6
Write-Output $out
