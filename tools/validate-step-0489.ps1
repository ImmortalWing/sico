# Validates STEP-0489 (M23 residual closure A — diagnostics protocol full
# extension):
#  1. the protocol validator accepts the catalog plus the implementation
#     corpus (design-level P0 cases + implementation-level cases);
#  2. every registered implementation case is reproduced by the REAL compiler:
#     `sico check --json` must emit the catalog code with the catalog key, the
#     catalog argument set, and the catalog-rendered default message — byte for
#     byte;
#  3. the implementation corpus manifest matches the corpus tree;
#  4. the working tree has no whitespace damage.
# Run from the repository root:
#   powershell -NoProfile -ExecutionPolicy Bypass -File tools/validate-step-0489.ps1
# (ErrorActionPreference stays default: this validator invokes native tools
# whose stderr streams must not become terminating errors; failures are
# signalled explicitly with `throw`.)

$repo = Split-Path -Parent $PSScriptRoot
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { throw 'cargo not found' }
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

function Read-Json([string]$Path) {
  return Get-Content -LiteralPath $Path -Encoding UTF8 -Raw | ConvertFrom-Json
}

function Render-Message([string]$Template, $Arguments) {
  $rendered = $Template
  foreach ($property in @($Arguments.PSObject.Properties)) {
    $value = $property.Value
    if ($value -is [Array]) { $value = (@($value) -join ', ') }
    $rendered = $rendered.Replace("{$($property.Name)}", [string]$value)
  }
  return $rendered
}

Write-Host 'running the protocol validator...'
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $repo 'tools/validate-diagnostics.ps1')
if ($LASTEXITCODE -ne 0) { throw 'the diagnostics protocol validator failed' }

Write-Host 'building the sico CLI...'
& $cargo build --locked --offline --quiet -p sico-cli
if ($LASTEXITCODE -ne 0) { throw 'sico-cli build failed' }
$sico = Join-Path $repo 'target/debug/sico.exe'
if (-not (Test-Path $sico)) { throw "sico CLI not found: $sico" }

$catalog = Read-Json (Join-Path $repo 'diagnostics/catalog.json')
$caseMap = Read-Json (Join-Path $repo 'diagnostics/semantic-case-map.json')
$catalogByCode = @{}
foreach ($definition in @($catalog.diagnostics)) {
  $catalogByCode[[string]$definition.code] = $definition
}

$implementationRoot = Join-Path $repo 'diagnostics/implementation-cases'
$implementationManifest = Read-Json (Join-Path $implementationRoot 'manifest.json')
$implementationCases = @($caseMap.cases | Where-Object {
  ([string]$_.source).StartsWith('diagnostics/implementation-cases/')
})

Write-Host 'comparing the catalog against the real compiler...'
$checked = 0
foreach ($case in $implementationCases) {
  $caseId = [string]$case.case
  $code = [string]$case.code
  $key = [string]$case.key
  $source = Join-Path $repo ([string]$case.source)
  if (-not (Test-Path -LiteralPath $source)) { throw "missing case source: $source" }
  if (-not $catalogByCode.ContainsKey($code)) { throw "case code is not in the catalog: $caseId -> $code" }
  $definition = $catalogByCode[$code]
  if ([string]$definition.key -ne $key) { throw "case key differs from the catalog: $caseId" }

  $psi = New-Object System.Diagnostics.ProcessStartInfo
  $psi.FileName = $sico
  $psi.Arguments = "check --json `"$source`""
  $psi.UseShellExecute = $false
  $psi.RedirectStandardOutput = $true
  $psi.RedirectStandardError = $true
  $psi.StandardOutputEncoding = [Text.Encoding]::UTF8
  $psi.StandardErrorEncoding = [Text.Encoding]::UTF8
  $process = [System.Diagnostics.Process]::Start($psi)
  $stdout = $process.StandardOutput.ReadToEnd()
  $stderr = $process.StandardError.ReadToEnd()
  [void]$process.WaitForExit()
  if ($process.ExitCode -ne 1) {
    throw "case $caseId must be refused with exit 1, got $($process.ExitCode): $stderr"
  }
  $envelope = $stdout | ConvertFrom-Json
  $emitted = @($envelope.diagnostics | Where-Object { [string]$_.code -eq $code })
  if ($emitted.Count -ne 1) {
    throw "case $caseId must emit exactly one $code diagnostic, got $($emitted.Count)"
  }
  $diagnostic = $emitted[0]
  if ([string]$diagnostic.key -ne $key) {
    throw "case $caseId key differs: expected $key, got $($diagnostic.key)"
  }

  $required = @($definition.required_arguments | Sort-Object)
  $emittedNames = @($diagnostic.arguments.PSObject.Properties | ForEach-Object { $_.Name } | Sort-Object)
  if (($required -join ',') -ne ($emittedNames -join ',')) {
    throw "case $caseId argument set differs: expected [$($required -join ', ')], found [$($emittedNames -join ', ')]"
  }
  foreach ($name in $required) {
    $expectedValue = [string]$case.arguments.PSObject.Properties[$name].Value
    $emittedValue = [string]$diagnostic.arguments.PSObject.Properties[$name].Value
    if ($expectedValue -ne $emittedValue) {
      throw "case $caseId argument '$name' differs: expected '$expectedValue', got '$emittedValue'"
    }
  }

  $rendered = Render-Message ([string]$definition.message_template) $case.arguments
  if ([string]$diagnostic.message -ne $rendered) {
    throw "case $caseId message differs: expected '$rendered', got '$($diagnostic.message)'"
  }
  if ([string]$case.expected_message -ne $rendered) {
    throw "case $caseId case-map message differs from the catalog render: '$rendered'"
  }
  $checked++
  Write-Host ("  {0} {1} ok" -f $caseId, $code)
}

if ($checked -ne [int]$implementationManifest.case_count) {
  throw "checked $checked implementation cases but the manifest declares $($implementationManifest.case_count)"
}

Write-Host 'checking the working tree whitespace...'
& git -C $repo diff --check
if ($LASTEXITCODE -ne 0) { throw 'git diff --check reported whitespace damage' }

Write-Host "STEP-0489 validator OK (protocol validator, $checked implementation cases reproduced by the real compiler, diff-check)"
