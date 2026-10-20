# Bare-literal ceremony measurement (M23 plan §8.3 item 3, selfhost/AI legs).
#
# Pure deterministic text measurement over a corpus of .sico sources. No
# grammar, IR or product code changes; zero network access.
#
# Schema: sico.bare-literal-ceremony.v0
#
# Metric definitions (each field names its exact pattern so an independent
# run recomputes it):
#   literal_constructor_calls  '\b(I64|U64|F64|Bool)\.literal\(' occurrences
#                              (the same pattern the STEP-0284 ceremony census
#                              uses, so the two tools cross-check each other)
#   call_sites                '\b[A-Za-z_][A-Za-z0-9_.]*\s*\(' on expression
#                              lines; the natural denominator for "ceremony per
#                              expression", because a literal constructor IS a
#                              call site
#   expression_lines          non-blank, non-comment lines that are not pure
#                              declarations (function/record/enum/case/end/
#                              field/use/module/capability/resource/task/
#                              interface/using) — a textual proxy, NOT a parse
#                              count; the same discipline as the STEP-0284
#                              census
#   bytes / kb                UTF-8 bytes of the corpus and bytes/1024
#   typed_literal_form        the same corpus rewritten so that every
#                              `X.literal(<digits>)` becomes `<digits>`; the
#                              ceremony calls there are 0 by construction and
#                              the byte/line delta is what the explicit-width
#                              form costs
#
# Usage:
#   powershell -NoProfile -ExecutionPolicy Bypass `
#     -File tools/measure-bare-literal-ceremony.ps1 `
#     -CorpusPath selfhost -OutputPath docs/reports/bare-literal-ceremony-<date>.json
#
param(
    [Parameter(Mandatory = $true)]
    [string]$CorpusPath,
    [string]$OutputPath = '',
    [string]$RepositoryRoot = '',
    [switch]$Quiet
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

if (-not $RepositoryRoot) {
    # $PSScriptRoot is not always populated when the script is launched
    # through `powershell -File` with an explicit parameter set, so resolve the
    # repository root from the invocation path instead.
    $scriptDirectory = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
    $RepositoryRoot = Split-Path -Parent $scriptDirectory
}

$root = (Resolve-Path $RepositoryRoot).Path
$corpus = if ([IO.Path]::IsPathRooted($CorpusPath)) { $CorpusPath } else { Join-Path $root $CorpusPath }
if (-not (Test-Path -LiteralPath $corpus)) { throw "corpus path not found: $corpus" }
$corpusRoot = (Resolve-Path $corpus).Path

$files = @(Get-ChildItem -LiteralPath $corpusRoot -Recurse -Filter '*.sico' -File | Sort-Object FullName)
if ($files.Count -eq 0) { throw "no .sico sources under $corpusRoot" }

$literalPattern = '\b(I64|U64|F64|Bool)\.literal\('
$callPattern = '\b[A-Za-z_][A-Za-z0-9_.]*\s*\('
$declarationPattern = '^\s*(function|record|enum|case|end|field|use|module|capability|resource|task|interface|using)\b'
$literalRewritePattern = '\b(?:I64|U64|F64|Bool)\.literal\((-?\d+)\)'

$totalBytes = 0
$totalLines = 0
$totalExpressionLines = 0
$totalCallSites = 0
$totalLiteralCalls = 0
$typedBytes = 0
$typedLines = 0
$typedExpressionLines = 0
$typedCallSites = 0
$perFile = @()

foreach ($file in $files) {
    $text = [IO.File]::ReadAllText($file.FullName)
    $lines = [IO.File]::ReadAllLines($file.FullName)
    $bytes = (Get-Item -LiteralPath $file.FullName).Length

    $literalCalls = [regex]::Matches($text, $literalPattern).Count
    $expressionLines = 0
    $callSites = 0
    foreach ($line in $lines) {
        $trimmed = $line.Trim()
        if ($trimmed.Length -eq 0) { continue }
        if ($trimmed.StartsWith('//')) { continue }
        if ($trimmed -match $declarationPattern) { continue }
        $expressionLines++
        $callSites += [regex]::Matches($line, $callPattern).Count
    }

    $typedText = [regex]::Replace($text, $literalRewritePattern, '$1')
    $typedFileLines = [IO.File]::ReadAllLines($file.FullName)
    $typedExpressionLines = 0
    $typedFileCallSites = 0
    for ($i = 0; $i -lt $typedFileLines.Count; $i++) {
        $trimmed = $typedFileLines[$i].Trim()
        if ($trimmed.Length -eq 0) { continue }
        if ($trimmed.StartsWith('//')) { continue }
        if ($trimmed -match $declarationPattern) { continue }
        $typedExpressionLines++
        $typedFileCallSites += [regex]::Matches($typedFileLines[$i], $callPattern).Count
    }
    $typedFileBytes = [Text.Encoding]::UTF8.GetByteCount($typedText)

    $totalBytes += $bytes
    $totalLines += $lines.Count
    $totalExpressionLines += $expressionLines
    $totalCallSites += $callSites
    $totalLiteralCalls += $literalCalls
    $typedBytes += $typedFileBytes
    $typedLines += $typedFileLines.Count
    $typedExpressionLines += $typedExpressionLines
    $typedCallSites += $typedFileCallSites

    $perFile += [ordered]@{
        file = $file.FullName.Substring($corpusRoot.Length + 1).Replace('\', '/')
        bytes = $bytes
        lines = $lines.Count
        expression_lines = $expressionLines
        call_sites = $callSites
        literal_constructor_calls = $literalCalls
        typed_literal_bytes = $typedFileBytes
    }
}

$kb = [Math]::Round($totalBytes / 1024, 3)
$typedKb = [Math]::Round($typedBytes / 1024, 3)

$report = [ordered]@{
    schema = 'sico.bare-literal-ceremony.v0'
    corpus = [ordered]@{
        root = $corpusRoot.Replace('\', '/')
        files = $files.Count
        bytes = $totalBytes
        kb = $kb
        lines = $totalLines
        expression_lines = $totalExpressionLines
        call_sites = $totalCallSites
        literal_constructor_calls = $totalLiteralCalls
    }
    densities = [ordered]@{
        literal_calls_per_kb = [Math]::Round($totalLiteralCalls / $kb, 3)
        literal_calls_per_100_expression_lines = [Math]::Round(100 * $totalLiteralCalls / $totalExpressionLines, 3)
        literal_calls_per_100_call_sites = [Math]::Round(100 * $totalLiteralCalls / $totalCallSites, 3)
    }
    typed_literal_form = [ordered]@{
        bytes = $typedBytes
        kb = $typedKb
        lines = $typedLines
        expression_lines = $typedExpressionLines
        call_sites = $typedCallSites
        literal_constructor_calls = 0
        bytes_saved = $totalBytes - $typedBytes
        bytes_saved_ratio = [Math]::Round(($totalBytes - $typedBytes) / $totalBytes, 6)
    }
    pattern_notes = [ordered]@{
        literal_constructor_calls = $literalPattern
        call_sites = $callPattern
        expression_lines = 'non-blank, non-comment, non-declaration lines (textual proxy, not a parse count)'
        typed_literal_form = 'every X.literal(<digits>) rewritten to <digits>'
    }
    honesty = [ordered]@{
        evidence_class = 'internal-fixture'
        measurement = 'deterministic text census; no model call, no grammar change'
        limitation = 'expression lines are a textual proxy; a parse-level count would be required before any grammar proposal'
    }
    per_file = $perFile
}

if ($OutputPath) {
    $out = if ([IO.Path]::IsPathRooted($OutputPath)) { $OutputPath } else { Join-Path $root $OutputPath }
    $parent = Split-Path -Parent $out
    if ($parent -and -not (Test-Path -LiteralPath $parent)) { New-Item -ItemType Directory -Force -Path $parent | Out-Null }
    [IO.File]::WriteAllText($out, ($report | ConvertTo-Json -Depth 8) + "`n", [Text.UTF8Encoding]::new($false))
}

if (-not $Quiet) {
    Write-Host ("BARE_LITERAL_CEREMONY files={0} lines={1} kb={2} expression_lines={3} call_sites={4} literals={5} per_kb={6} per_100_expr={7} typed_bytes_saved={8}" -f `
        $report.corpus.files, $report.corpus.lines, $report.corpus.kb, $report.corpus.expression_lines, `
        $report.corpus.call_sites, $report.corpus.literal_constructor_calls, `
        $report.densities.literal_calls_per_kb, $report.densities.literal_calls_per_100_expression_lines, `
        $report.typed_literal_form.bytes_saved)
}
