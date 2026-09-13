# M19 §3.2: reproducible-build rehearsal — the same source built twice
# through fully independent cache dirs must produce byte-identical
# Components. Evidence lands in target/evidence/m19/.
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
$repo = Split-Path -Parent $PSScriptRoot
Set-Location $repo
[Console]::OutputEncoding = [Text.Encoding]::UTF8

$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& $cargo build -q -p sico-cli --offline
if ($LASTEXITCODE -ne 0) { throw 'sico build failed' }
$sico = Join-Path $repo 'target\debug\sico.exe'

$sources = @('tests/end-to-end/script-word-count.sico', 'tests/end-to-end/map-set-frequency.sico')
$results = @()
foreach ($source in $sources) {
    $digests = @()
    foreach ($round in 'a', 'b') {
        $dir = Join-Path $env:TEMP ("sico-repro-{0}-{1}-{2}" -f ($source -replace '[/\\.]', '-'), $round, $PID)
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        $env:SICO_CACHE_DIR = Join-Path $dir 'cache'
        $out = Join-Path $dir 'out.component.wasm'
        & $sico build --profile script-v0 --output $out $source | Out-Null
        if ($LASTEXITCODE -ne 0) { throw "build failed: $source (round $round)" }
        $sha = (Get-FileHash $out -Algorithm SHA256).Hash.ToLower()
        $digests += $sha
        Remove-Item -Recurse -Force $dir
    }
    $match = $digests[0] -eq $digests[1]
    $name = Split-Path -Leaf $source
    Write-Host "$name : $($digests[0]) match=$match"
    if (-not $match) { throw "reproducibility broken: $name" }
    $results += @{ source = $name; sha256 = $digests[0] }
}
Remove-Item Env:\SICO_CACHE_DIR -ErrorAction SilentlyContinue

$evidenceDir = Join-Path $repo 'target\evidence\m19'
New-Item -ItemType Directory -Force -Path $evidenceDir | Out-Null
@{
    schema = 'sico.m19.reproducible-build.v0'
    date = (Get-Date -Format 'yyyy-MM-dd')
    method = 'two independent SICO_CACHE_DIRs, same source, byte-compare'
    components = $results
    all_match = $true
} | ConvertTo-Json -Depth 3 | Set-Content (Join-Path $evidenceDir 'reproducible-build.json')
Write-Host 'reproducible-build rehearsal: OK (evidence written)'
