param(
  [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = 'Stop'

function Read-NormalizedText([string]$Path) {
  $text = [IO.File]::ReadAllText($Path, [Text.Encoding]::UTF8)
  return $text.Replace("`r`n", "`n").Replace("`r", "`n")
}

function Read-Header([string]$Text, [string]$Name, [string]$Path) {
  $match = [regex]::Match($Text, "(?m)^// $([regex]::Escape($Name)): (.+)$")
  if (-not $match.Success) {
    throw "$Path is missing metadata '$Name'"
  }
  return $match.Groups[1].Value
}

function Get-RelativePath([string]$BasePath, [string]$Path) {
  $base = [IO.Path]::GetFullPath($BasePath).TrimEnd('\', '/') + [IO.Path]::DirectorySeparatorChar
  $full = [IO.Path]::GetFullPath($Path)
  if (-not $full.StartsWith($base, [StringComparison]::OrdinalIgnoreCase)) {
    throw "path is outside base: $Path"
  }
  return $full.Substring($base.Length).Replace('\', '/')
}

$root = (Resolve-Path $RepositoryRoot).Path
$a0Root = Join-Path $root 'semantic-cases'
$candidateRoots = @{
  B = Join-Path $root 'syntax-candidates/b'
  C = Join-Path $root 'syntax-candidates/c'
}
$manifest = Get-Content -LiteralPath (Join-Path $a0Root 'manifest.json') -Encoding UTF8 -Raw | ConvertFrom-Json
if ($manifest.schema_version -ne 1) {
  throw "unsupported semantic case manifest schema: $($manifest.schema_version)"
}
$knownSemantics = @{}
$semanticsDocument = Read-NormalizedText (Join-Path $root 'SEMANTICS.md')
foreach ($match in [regex]::Matches($semanticsDocument, '(?m)^### (SEM-[0-9][0-9][0-9])')) {
  $knownSemantics[$match.Groups[1].Value] = $true
}
if ($knownSemantics.Count -eq 0) {
  throw 'no semantic rule ids found in SEMANTICS.md'
}

$groups = @($manifest.groups)
$groupIds = @{}
$expectedCaseCount = 0
$expectedValidCount = 0
$expectedInvalidCount = 0
foreach ($group in $groups) {
  if ($groupIds.ContainsKey([string]$group.id)) {
    throw "duplicate semantic group: $($group.id)"
  }
  $groupIds[[string]$group.id] = $true
  $groupRoot = Join-Path $a0Root $group.id
  if (-not (Test-Path -LiteralPath (Join-Path $groupRoot 'README.md'))) {
    throw "semantic group is missing README: $($group.id)"
  }
  $validFiles = @(Get-ChildItem (Join-Path $groupRoot 'valid') -Filter '*.sico' -File)
  $invalidFiles = @(Get-ChildItem (Join-Path $groupRoot 'invalid') -Filter '*.sico' -File)
  if ($validFiles.Count -ne [int]$group.valid) {
    throw "expected $($group.valid) valid files for $($group.id), found $($validFiles.Count)"
  }
  if ($invalidFiles.Count -ne [int]$group.invalid) {
    throw "expected $($group.invalid) invalid files for $($group.id), found $($invalidFiles.Count)"
  }
  $expectedValidCount += [int]$group.valid
  $expectedInvalidCount += [int]$group.invalid
  $expectedCaseCount += [int]$group.valid + [int]$group.invalid
}

if ($expectedCaseCount -ne [int]$manifest.case_count -or
    $expectedValidCount -ne [int]$manifest.valid_count -or
    $expectedInvalidCount -ne [int]$manifest.invalid_count) {
  throw 'semantic manifest totals do not match group totals'
}

$a0Files = @(Get-ChildItem $a0Root -Recurse -Filter '*.sico' -File)
if ($a0Files.Count -ne [int]$manifest.case_count) {
  throw "expected $($manifest.case_count) A0 cases, found $($a0Files.Count)"
}
$caseIds = @{}
$registeredRelativePaths = @{}

foreach ($a0File in $a0Files) {
  $relativePath = Get-RelativePath $a0Root $a0File.FullName
  $parts = $relativePath -split '/'
  if ($parts.Count -ne 3 -or -not $groupIds.ContainsKey($parts[0]) -or $parts[1] -notin @('valid', 'invalid')) {
    throw "unexpected semantic case path: $relativePath"
  }
  $group = $groups | Where-Object id -eq $parts[0]
  $a0Text = Read-NormalizedText $a0File.FullName
  $caseId = Read-Header $a0Text 'case' $relativePath
  $expect = Read-Header $a0Text 'expect' $relativePath
  $semantics = Read-Header $a0Text 'semantics' $relativePath
  if ($caseId -notmatch "^$([regex]::Escape([string]$group.prefix))-\d{3}$") {
    throw "case id does not match group prefix: $relativePath -> $caseId"
  }
  if ($caseIds.ContainsKey($caseId)) {
    throw "duplicate A0 case id: $caseId"
  }
  $caseIds[$caseId] = $true
  if ($semantics -notmatch '^SEM-\d{3}(, SEM-\d{3})*$') {
    throw "invalid semantics metadata: $relativePath -> $semantics"
  }
  foreach ($semanticId in @($semantics -split ', ')) {
    if (-not $knownSemantics.ContainsKey($semanticId)) {
      throw "unknown semantic rule in ${relativePath}: $semanticId"
    }
  }
  if ($parts[1] -eq 'valid' -and $expect -ne 'accept') {
    throw "valid case must expect accept: $relativePath"
  }
  if ($parts[1] -eq 'invalid' -and $expect -notmatch '^reject\([A-Z][A-Z0-9_]*\)$') {
    throw "invalid case must expect a diagnostic key: $relativePath"
  }
  if ([regex]::IsMatch($a0Text, '(?m)^// syntax:')) {
    throw "A0 case must not contain syntax metadata: $relativePath"
  }
  $registeredRelativePaths[$relativePath] = $true

  foreach ($syntax in @('B', 'C')) {
    $candidatePath = Join-Path $candidateRoots[$syntax] $relativePath
    if (-not (Test-Path -LiteralPath $candidatePath)) {
      throw "missing $syntax mirror: $relativePath"
    }
    $candidateText = Read-NormalizedText $candidatePath
    if ((Read-Header $candidateText 'case' "$syntax/$relativePath") -ne $caseId -or
        (Read-Header $candidateText 'syntax' "$syntax/$relativePath") -ne $syntax -or
        (Read-Header $candidateText 'expect' "$syntax/$relativePath") -ne $expect -or
        (Read-Header $candidateText 'semantics' "$syntax/$relativePath") -ne $semantics) {
      throw "$syntax metadata mismatch: $relativePath"
    }
  }
}

foreach ($syntax in @('B', 'C')) {
  $candidateFiles = @(Get-ChildItem $candidateRoots[$syntax] -Recurse -Filter '*.sico' -File)
  if ($candidateFiles.Count -ne [int]$manifest.case_count) {
    throw "expected $($manifest.case_count) $syntax cases, found $($candidateFiles.Count)"
  }
  foreach ($file in $candidateFiles) {
    $relativePath = Get-RelativePath $candidateRoots[$syntax] $file.FullName
    if (-not $registeredRelativePaths.ContainsKey($relativePath)) {
      throw "unregistered $syntax case: $relativePath"
    }
  }
}

if ([int]$manifest.candidate_count -ne 3 -or [int]$manifest.program_count -ne [int]$manifest.case_count * 3) {
  throw 'semantic manifest candidate/program totals are invalid'
}

Write-Output "SEMANTIC_CASES_OK cases=$($manifest.case_count) valid=$($manifest.valid_count) invalid=$($manifest.invalid_count) A0=$($manifest.case_count) B=$($manifest.case_count) C=$($manifest.case_count) programs=$($manifest.program_count)"
