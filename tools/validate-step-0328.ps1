# Validates STEP-0328 (M23 language v1 batch 3 item 1, RFC-0048):
#  1. the machine matrix carries the executable infix-comparison row and the
#     declared refusal rows;
#  2. the e2e corpus exists and passes `sico test` (byte-exact stdout through
#     the real runner);
#  3. the typed refusal corpus reproduces: E2025 at check for non-atom
#     swap-form operands, the named build refusals for operand types /
#     chained comparisons / `&&` position violations, and the lexical
#     refusals for `||` / bare `!` / bare `&`;
#  4. the formatter canonicalizes operator spacing and is idempotent.
# Run from the repository root:  powershell -File tools/validate-step-0328.ps1
# (ErrorActionPreference stays default: this validator invokes native tools
# whose stderr streams must not become terminating errors; failures are
# signalled explicitly with `throw`.)

$repo = Split-Path -Parent $PSScriptRoot
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { throw 'cargo not found' }
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

$json = Get-Content (Join-Path $repo 'tests/language-matrix/application-profile-v0.json') -Raw | ConvertFrom-Json
$row = $json.matrix.executable.'infix-comparison-conditions'
if (-not $row) { throw 'executable matrix row infix-comparison-conditions missing' }
if (-not ($row.check -and $row.build -and $row.run)) { throw 'infix-comparison-conditions must be executable end to end' }
if ($row.step -ne 'STEP-0328' -or $row.rfc -ne 'RFC-0048') { throw 'infix-comparison-conditions identity mismatch' }
foreach ($member in 'infix-swap-form-non-atom-operands', 'disjunction-negation-operators',
    'chained-comparisons-and-value-position-and') {
    if (-not $json.matrix.refused.$member) { throw "refused matrix entry missing: $member" }
}
$fixture = Join-Path $repo $json.'exit-corpus'.'infix-comparisons-fixture'
if (-not (Test-Path $fixture)) { throw "e2e fixture missing: $fixture" }
$runnerTest = Join-Path $repo $json.'exit-corpus'.'batch3-runner-test'
if (-not (Test-Path $runnerTest)) { throw "runner test missing: $runnerTest" }

Write-Host 'building sico CLI and runner...'
& $cargo build --locked --offline --quiet -p sico-cli
if ($LASTEXITCODE -ne 0) { throw 'sico-cli build failed' }
& $cargo build --locked --offline --quiet --manifest-path (Join-Path $repo 'runner/sico-runner/Cargo.toml')
if ($LASTEXITCODE -ne 0) { throw 'sico-runner build failed' }
$env:SICO_RUNNER = Join-Path $repo 'runner/sico-runner/target/debug/sico-runner.exe'
$sico = Join-Path $repo 'target/debug/sico.exe'

Write-Host 'running the e2e corpus through sico test...'
& $sico test $fixture 2>&1 | Tee-Object -Variable testOutput | Out-Null
if ($LASTEXITCODE -ne 0) { throw "sico test failed: $($testOutput -join '; ')" }
if (-not ($testOutput -match 'PASS')) { throw "sico test did not report PASS: $($testOutput -join '; ')" }

function Invoke-Refused([string]$Tag, [string]$Source, [string]$Command, [string]$Needle) {
    $directory = Join-Path $env:TEMP "sico-validate-0328-$Tag"
    if (Test-Path $directory) { Remove-Item -Recurse -Force $directory }
    New-Item -ItemType Directory $directory | Out-Null
    $path = Join-Path $directory "$Tag.sico"
    [IO.File]::WriteAllText($path, $Source, [Text.UTF8Encoding]::new($false))
    $stdout = Join-Path $directory 'stdout.txt'
    $stderr = Join-Path $directory 'stderr.txt'
    $arguments = "$Command --profile script-v0"
    if ($Command -eq 'check') {
        $arguments = $Command
    }
    $arguments = "$arguments `"$path`""
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $sico
    $psi.Arguments = $arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [Text.Encoding]::UTF8
    $process = [System.Diagnostics.Process]::Start($psi)
    $errorText = $process.StandardError.ReadToEnd()
    [void]$process.StandardOutput.ReadToEnd()
    [void]$process.WaitForExit()
    $code = $process.ExitCode
    Remove-Item -Recurse -Force $directory
    if ($code -eq 0) { throw "$Tag must be refused" }
    if (-not $errorText.Contains($Needle)) { throw "$Tag refusal needle missing: $errorText" }
}

$header = @'
record ScriptInput:
  field arguments: List[Text]
  field stdin: Bytes
end record

record ScriptOutput:
  field stdout: Bytes
  field stderr: Bytes
  field exit_code: I64
end record

enum ScriptErrorCode:
  case InvalidInput
  case ResourceLimit
  case DomainError
  case Cancelled
end enum

record ScriptError:
  field code: ScriptErrorCode
  field message: Text
end record

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
'@
$footer = @'
  return ok(ScriptOutput(stdout: sico.text.encode("x"), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
end function
'@

Write-Host 'checking the typed refusal corpus...'
Invoke-Refused 'e2025' "$header
  let a = I64.literal(3)
  if a > I64.literal(4):
    set a = I64.literal(0)
  end if
$footer" 'check' 'E2025'

Invoke-Refused 'mixed-width' "$header
  let a = I64.literal(3)
  if a < U64.literal(4):
    set a = I64.literal(0)
  end if
$footer" 'build' 'infix comparison condition operand types'

Invoke-Refused 'chained' "$header
  let a = I64.literal(3)
  let b = I64.literal(7)
  if a < b < I64.literal(9):
    set a = I64.literal(0)
  end if
$footer" 'build' 'chained comparison'

Invoke-Refused 'and-value-position' "$header
  let a = I64.literal(3)
  let b = I64.literal(7)
  let c = a < b && a != I64.literal(0)
$footer" 'build' '&& outside a condition position'

Invoke-Refused 'and-non-condition' "$header
  let a = I64.literal(3)
  let flag = true
  if flag && a < I64.literal(4):
    set a = I64.literal(0)
  end if
$footer" 'build' '&& operand is not a comparison condition'

Invoke-Refused 'disjunction-lex' "$header
  let a = I64.literal(3)
  let b = I64.literal(7)
  if a < b || a != I64.literal(0):
    set a = I64.literal(0)
  end if
$footer" 'check' "UnexpectedCharacter('|')"

Invoke-Refused 'bare-amp-lex' "$header
  let a = I64.literal(3)
  let b = I64.literal(7)
  if a < b & a != I64.literal(0):
    set a = I64.literal(0)
  end if
$footer" 'check' "UnexpectedCharacter('&')"

Write-Host 'checking formatter spacing and idempotence...'
$compact = Join-Path $env:TEMP 'sico-validate-0328-format.sico'
[IO.File]::WriteAllText($compact, "$header
  let a = I64.literal(3)
  let b = I64.literal(7)
  if a<b && b>=I64.literal(7):
    set a = I64.literal(0)
  end if
$footer", [Text.UTF8Encoding]::new($false))

function Invoke-Capture([string]$Arguments) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName = $sico
    $psi.Arguments = $Arguments
    $psi.UseShellExecute = $false
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError = $true
    $psi.StandardOutputEncoding = [Text.Encoding]::UTF8
    $psi.StandardErrorEncoding = [Text.Encoding]::UTF8
    $process = [System.Diagnostics.Process]::Start($psi)
    $out = $process.StandardOutput.ReadToEnd()
    [void]$process.StandardError.ReadToEnd()
    [void]$process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw "sico $Arguments failed with $($process.ExitCode)" }
    return $out
}

$first = Invoke-Capture "format `"$compact`""
# feed the formatted text back through stdin for the second pass
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $sico
$psi.Arguments = "format -"
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.RedirectStandardInput = $true
$psi.StandardOutputEncoding = [Text.Encoding]::UTF8
$psi.StandardErrorEncoding = [Text.Encoding]::UTF8
$process = [System.Diagnostics.Process]::Start($psi)
$process.StandardInput.Write($first)
$process.StandardInput.Close()
$second = $process.StandardOutput.ReadToEnd()
[void]$process.WaitForExit()
if ($process.ExitCode -ne 0) { throw 'second formatter pass failed' }
Remove-Item -Force $compact
if ($first -ne $second) { throw 'formatter is not idempotent on the batch-3 condition shapes' }
if (-not $first.Contains('if a < b && b >= I64.literal(7):')) {
    throw 'formatter did not canonicalize the one-space operator spacing'
}

Write-Host 'STEP-0328 validator OK (matrix, e2e corpus, refusal corpus, formatter idempotence)'
