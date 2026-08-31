param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
  [string]$CargoPath = (Join-Path $HOME '.cargo/bin/cargo.exe')
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

function Read-Utf8([string]$Path) {
  return Get-Content -LiteralPath $Path -Raw -Encoding UTF8
}

function Read-Metadata([string]$Text, [string]$Name, [string]$Context) {
  $match = [regex]::Match($Text, "(?m)^> - $([regex]::Escape($Name)): ([^\r\n]+)$")
  if (-not $match.Success) {
    throw "$Context is missing metadata '$Name'"
  }
  return $match.Groups[1].Value.Trim()
}

$root = (Resolve-Path $RepositoryRoot).Path
$contractPath = Join-Path $root 'tests/lexical/contract-v0.json'
$rfcPath = Join-Path $root 'docs/rfc/RFC-0006-lexical-source-contract-v0.md'
$stepPath = Join-Path $root 'docs/steps/STEP-0015-compiler-workspace-lexical-source.md'

foreach ($path in @($contractPath, $rfcPath, $stepPath, (Join-Path $root 'Cargo.toml'), (Join-Path $root 'Cargo.lock'))) {
  if (-not (Test-Path -LiteralPath $path)) {
    throw "missing STEP-0015 artifact: $path"
  }
}

$contract = Read-Utf8 $contractPath | ConvertFrom-Json
if ($contract.schema -ne 'sico.lexical-contract.v0' -or $contract.unicode_version -ne '17.0.0') {
  throw 'unexpected lexical contract schema or Unicode version'
}
if ($contract.positive.Count -ne 7 -or $contract.negative.Count -ne 14) {
  throw "expected 7 positive and 14 negative cases, found $($contract.positive.Count)/$($contract.negative.Count)"
}

$allCases = @($contract.positive) + @($contract.negative)
$duplicateIds = @($allCases | Group-Object id | Where-Object Count -ne 1)
if ($duplicateIds.Count -ne 0) {
  throw "duplicate lexical case id: $($duplicateIds[0].Name)"
}
foreach ($side in @('positive', 'negative')) {
  $categories = @($contract.$side | ForEach-Object category | Sort-Object -Unique)
  foreach ($required in @('identifier', 'limit', 'newline', 'span', 'token', 'trivia', 'utf8')) {
    if ($required -notin $categories) {
      throw "$side cases do not cover category '$required'"
    }
  }
}

$limits = [ordered]@{
  source_bytes = 16777216
  line_bytes = 1048576
  identifier_bytes = 1024
  token_bytes = 1048576
  tokens = 1000000
  diagnostics = 100
}
foreach ($name in $limits.Keys) {
  if ($contract.limits.$name -ne $limits[$name]) {
    throw "unexpected $name limit: $($contract.limits.$name)"
  }
}

$spanCase = $contract.positive | Where-Object id -eq 'LEX-P006'
$expectedPoints = @('0:1:1', '3:1:4', '7:1:5', '8:2:1', '11:2:2')
$actualPoints = @($spanCase.points | ForEach-Object { "$($_.byte):$($_.line):$($_.column)" })
if (($actualPoints -join ',') -ne ($expectedPoints -join ',')) {
  throw 'LEX-P006 span oracle changed unexpectedly'
}
if ([Text.Encoding]::UTF8.GetByteCount($spanCase.source) -ne 11) {
  throw 'LEX-P006 source is not 11 UTF-8 bytes'
}

$nonNfc = ($contract.negative | Where-Object id -eq 'LEX-N003').source
if ($nonNfc.IsNormalized([Text.NormalizationForm]::FormC)) {
  throw 'LEX-N003 must remain a non-NFC spelling'
}
$invalidBytes = [byte[]](($contract.negative | Where-Object id -eq 'LEX-N001').bytes)
$strictUtf8 = [Text.UTF8Encoding]::new($false, $true)
$invalidRejected = $false
try {
  [void]$strictUtf8.GetString($invalidBytes)
} catch [Text.DecoderFallbackException] {
  $invalidRejected = $true
}
if (-not $invalidRejected) {
  throw 'LEX-N001 bytes unexpectedly decode as UTF-8'
}

$rfc = Read-Utf8 $rfcPath
if ((Read-Metadata $rfc 'status' 'RFC-0006') -ne 'accepted') {
  throw 'RFC-0006 must be accepted before lexer implementation'
}
foreach ($requiredText in @(
  'unicode-ident 1.0.24',
  'unicode-normalization 0.1.25',
  'rowan 0.16.1',
  'clap 4.6.1',
  'proposed/not accepted',
  '16,777,216',
  '1,000,000'
)) {
  if (-not $rfc.Contains($requiredText)) {
    throw "RFC-0006 is missing contract text: $requiredText"
  }
}

$expectedCrates = @(
  'sico-cli',
  'sico-diagnostics',
  'sico-format',
  'sico-lexer',
  'sico-parser',
  'sico-source',
  'sico-syntax'
)
if (-not (Test-Path -LiteralPath $CargoPath)) {
  throw "cargo not found: $CargoPath"
}
$previousToolchain = $env:RUSTUP_TOOLCHAIN
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
try {
  $metadataJson = & $CargoPath metadata --offline --locked --format-version 1 2>$null
  if ($LASTEXITCODE -ne 0) {
    throw 'cargo metadata --locked failed'
  }
} finally {
  $env:RUSTUP_TOOLCHAIN = $previousToolchain
}
$metadata = $metadataJson | ConvertFrom-Json
$workspacePackages = @($metadata.packages | Where-Object { $_.id -in $metadata.workspace_members } | ForEach-Object name | Sort-Object)
if (@(Compare-Object $expectedCrates @($workspacePackages | Where-Object { $_ -in $expectedCrates })).Count -ne 0) {
  throw "STEP-0015 workspace crates are missing: $($workspacePackages -join ',')"
}
foreach ($package in $metadata.packages | Where-Object { $_.id -in $metadata.workspace_members }) {
  if ($package.version -ne '0.0.0' -or $package.edition -ne '2024' -or $package.rust_version -ne '1.97') {
    throw "unexpected package policy for $($package.name)"
  }
}
$registryPackages = @($metadata.packages | Where-Object source -like 'registry+*')
$lockedDirect = [ordered]@{
  clap = '4.6.1'
  rowan = '0.16.1'
  'text-size' = '1.1.1'
  'unicode-ident' = '1.0.24'
  'unicode-normalization' = '0.1.25'
}
foreach ($name in $lockedDirect.Keys) {
  $matches = @($registryPackages | Where-Object name -eq $name)
  if ($matches.Count -ne 1 -or $matches[0].version -ne $lockedDirect[$name]) {
    throw "dependency $name is not locked to $($lockedDirect[$name])"
  }
}

$mainFiles = @(Get-ChildItem -LiteralPath (Join-Path $root 'crates') -Recurse -File -Filter 'main.rs')
$step0020 = Join-Path $root 'docs/steps/STEP-0020-cli-check-format-outline.md'
if (-not (Test-Path -LiteralPath $step0020) -and $mainFiles.Count -ne 0) {
  throw 'STEP-0015 must not create a CLI binary before STEP-0020'
}

$sicoFiles = @(Get-ChildItem -LiteralPath $root -Recurse -File -Filter '*.sico' |
  Where-Object { $_.FullName -notmatch '[\\/]target[\\/]' })
$lf = 0
$crlf = 0
$bareCr = 0
$bom = 0
foreach ($file in $sicoFiles) {
  $bytes = [IO.File]::ReadAllBytes($file.FullName)
  try {
    [void]$strictUtf8.GetString($bytes)
  } catch [Text.DecoderFallbackException] {
    throw "fixture is not strict UTF-8: $($file.FullName)"
  }
  if ($bytes.Length -ge 3 -and $bytes[0] -eq 0xEF -and $bytes[1] -eq 0xBB -and $bytes[2] -eq 0xBF) {
    $bom++
  }
  for ($index = 0; $index -lt $bytes.Length; $index++) {
    if ($bytes[$index] -eq 13) {
      if ($index + 1 -lt $bytes.Length -and $bytes[$index + 1] -eq 10) {
        $crlf++
        $index++
      } else {
        $bareCr++
      }
    } elseif ($bytes[$index] -eq 10) {
      $lf++
    }
  }
}
if ($sicoFiles.Count -ne 208 -or $lf -ne 3979 -or $crlf -ne 0 -or $bareCr -ne 0 -or $bom -ne 0) {
  throw "unexpected corpus encoding baseline: files=$($sicoFiles.Count) lf=$lf crlf=$crlf bare_cr=$bareCr bom=$bom"
}

Write-Output "STEP_0015_OK crates=$($workspacePackages.Count) registry_dependencies=$($registryPackages.Count) cases=$($allCases.Count) corpus=$($sicoFiles.Count) lf=$lf crlf=$crlf bare_cr=$bareCr bom=$bom unicode=17.0.0"
