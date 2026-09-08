# Validates the STEP-0138 solver acceptance chain:
#  1. the frozen corpus parses and carries four fixtures with expected outputs;
#  2. the committed Python oracle reproduces every frozen expected output
#     (regeneration check; requires `python` on PATH);
#  3. the Sico port, the oracle script, the corpus and the runner test all exist.
# The Sico side's byte-exactness is proven continuously by
# runner/sico-runner/tests/block_solver.rs.
# Run from the repository root:  powershell -File tools/validate-step-0138.ps1
$ErrorActionPreference = 'Stop'

$repo = Split-Path -Parent $PSScriptRoot
$oracle = Join-Path $repo '案例项目/俄罗斯方块消除/oracle/oracle.py'
$corpus = Join-Path $repo '案例项目/俄罗斯方块消除/oracle/corpus.json'
$fixture = Join-Path $repo 'tests/end-to-end/block-solver.sico'
$runnerTest = Join-Path $repo 'runner/sico-runner/tests/block_solver.rs'

foreach ($path in @($oracle, $corpus, $fixture, $runnerTest)) {
    if (-not (Test-Path $path)) { throw "missing: $path" }
}

$json = Get-Content $corpus -Raw | ConvertFrom-Json
$names = @('trivial', 'medium', 'hard', 'limit')
foreach ($name in $names) {
    $entry = $json.$name
    if ($null -eq $entry) { throw "corpus fixture missing: $name" }
    if ($entry.input.board.Length -ne 64) { throw "$name board must be 64 chars" }
}

$probe = Join-Path $env:TEMP ("sico-step0138-{0}" -f $PID)
New-Item -ItemType Directory -Force -Path $probe | Out-Null
try {
    foreach ($name in $names) {
        $entry = $json.$name
        $inputPath = Join-Path $probe "$name.json"
        $doc = @{ board = $entry.input.board; tray = $entry.input.tray }
        $doc | ConvertTo-Json -Compress | Set-Content -Path $inputPath -Encoding UTF8
        $actual = Get-Content $inputPath -Raw | python $oracle | ForEach-Object { $_.Trim() }
        $expectedJson = $entry.expected | ConvertTo-Json -Compress
        $expectedFlat = ($expectedJson -replace '\s', '')
        if ($actual -ne $expectedFlat) {
            throw "${name}: oracle regeneration mismatch`n  expected: $expectedFlat`n  actual:   $actual"
        }
        Write-Host "${name}: oracle reproduces frozen output"
    }
} finally {
    Remove-Item -Recurse -Force $probe
}

Write-Host 'validate-step-0138: OK (corpus, oracle regeneration, artifacts)'
