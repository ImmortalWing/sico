# Validates STEP-0329 (M23 language v1 batch 3 item 2, RFC-0049):
#  1. the machine matrix carries the executable let-bound row and the
#     declared refusal rows;
#  2. the e2e corpus exists and passes `sico test` (byte-exact stdout through
#     the real runner);
#  3. the typed refusal corpus reproduces: E2026 (arm shape), E2027 (missing
#     else), E2001 (arm type mismatch) at check; the parse refusal for the
#     annotated form; the build refusal for a nested if-expression condition;
#  4. the let-bind nesting limit+1 refuses at the parse depth budget;
#  5. the formatter is idempotent with else:/case: at header depth.
# Run from the repository root:  powershell -File tools/validate-step-0329.ps1
# (ErrorActionPreference stays default: this validator invokes native tools
# whose stderr streams must not become terminating errors; failures are
# signalled explicitly with `throw`.)

$repo = Split-Path -Parent $PSScriptRoot
$cargo = "$env:USERPROFILE\.cargo\bin\cargo.exe"
if (-not (Test-Path $cargo)) { throw 'cargo not found' }
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'

$json = Get-Content (Join-Path $repo 'tests/language-matrix/application-profile-v0.json') -Raw | ConvertFrom-Json
$row = $json.matrix.executable.'let-bound-if-match'
if (-not $row) { throw 'executable matrix row let-bound-if-match missing' }
if (-not ($row.check -and $row.build -and $row.run)) { throw 'let-bound-if-match must be executable end to end' }
if ($row.step -ne 'STEP-0329' -or $row.rfc -ne 'RFC-0049') { throw 'let-bound-if-match identity mismatch' }
foreach ($member in 'let-bind-arm-shape-violations', 'expression-position-conditions') {
    if (-not $json.matrix.refused.$member) { throw "refused matrix entry missing: $member" }
}
$fixture = Join-Path $repo $json.'exit-corpus'.'let-bind-fixture'
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
    $err = $process.StandardError.ReadToEnd()
    [void]$process.WaitForExit()
    return @{ code = $process.ExitCode; stdout = $out; stderr = $err }
}

function Invoke-Refused([string]$Tag, [string]$Source, [string]$Command, [string]$Needle) {
    $directory = Join-Path $env:TEMP "sico-validate-0329-$Tag"
    if (Test-Path $directory) { Remove-Item -Recurse -Force $directory }
    New-Item -ItemType Directory $directory | Out-Null
    $path = Join-Path $directory "$Tag.sico"
    [IO.File]::WriteAllText($path, $Source, [Text.UTF8Encoding]::new($false))
    $arguments = "$Command --profile script-v0"
    if ($Command -eq 'check') {
        $arguments = $Command
    }
    $result = Invoke-Capture "$arguments `"$path`""
    Remove-Item -Recurse -Force $directory
    if ($result.code -eq 0) { throw "$Tag must be refused" }
    if (-not $result.stderr.Contains($Needle)) { throw "$Tag refusal needle missing: $($result.stderr)" }
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
Invoke-Refused 'lb-two-statements' "$header
  let a = I64.literal(3)
  let x = if a < I64.literal(4):
    set a = I64.literal(0)
    `"y`"
  else:
    `"n`"
  end if
$footer" 'check' 'E2026'

Invoke-Refused 'lb-return-arm' "$header
  let a = I64.literal(3)
  let x = if a < I64.literal(4):
    return error(ScriptError(code: ScriptErrorCode.DomainError, message: `"no`"))
  else:
    `"n`"
  end if
$footer" 'check' 'E2026'

Invoke-Refused 'lb-missing-else' "$header
  let a = I64.literal(3)
  let x = if a < I64.literal(4):
    `"y`"
  end if
$footer" 'check' 'E2027'

Invoke-Refused 'lb-arm-mismatch' "$header
  let a = I64.literal(3)
  let x = if a < I64.literal(4):
    `"y`"
  else:
    I64.literal(0)
  end if
$footer" 'check' 'expected Text, found I64'

Invoke-Refused 'lb-annotated' "$header
  let a = I64.literal(3)
  let x: I64 = if a < I64.literal(4):
    `"y`"
  else:
    `"n`"
  end if
$footer" 'check' 'MismatchedClose { expected: Function, actual: If }'

Invoke-Refused 'lb-nested-condition' "$header
  let a = I64.literal(3)
  let x = if (if a < I64.literal(4): true else: false end if):
    `"y`"
  else:
    `"n`"
  end if
$footer" 'build' 'core expression'

Write-Host 'checking the let-bind nesting budget...'
$levels = 255
$source = New-Object Text.StringBuilder
[void]$source.Append("$header")
[void]$source.Append("`n  let a = I64.literal(3)`n")
for ($level = 0; $level -lt $levels; $level++) {
    [void]$source.Append("  if a < I64.literal(7):`n")
    [void]$source.Append("  let t$level = if a < I64.literal(7):`n    `"y`"`n  else:`n    `"n`"`n  end if`n")
}
[void]$source.Append("  return ok(ScriptOutput(stdout: sico.text.encode(`"deep`"), stderr: sico.text.encode(`"`"), exit_code: I64.literal(0)))`n")
for ($level = 0; $level -lt $levels; $level++) {
    [void]$source.Append("  end if`n")
}
[void]$source.Append("  return error(ScriptError(code: ScriptErrorCode.DomainError, message: `"no`"))`nend function`n")
Invoke-Refused 'lb-nesting-over-limit' $source.ToString() 'check' 'NestingLimitExceeded { limit: 256 }'

Write-Host 'checking formatter canonical shape and idempotence...'
$once = Invoke-Capture "format `"$fixture`""
if ($once.code -ne 0) { throw 'formatting the let-bind fixture failed' }
if ($once.stdout -ne [IO.File]::ReadAllText($fixture).Replace("`r`n", "`n")) {
    throw 'the let-bind fixture is not in the canonical layout (else:/case: at header depth)'
}
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = $sico
$psi.Arguments = 'format -'
$psi.UseShellExecute = $false
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.RedirectStandardInput = $true
$psi.StandardOutputEncoding = [Text.Encoding]::UTF8
$psi.StandardErrorEncoding = [Text.Encoding]::UTF8
$process = [System.Diagnostics.Process]::Start($psi)
$bytes = [Text.Encoding]::UTF8.GetBytes($once.stdout)
$process.StandardInput.BaseStream.Write($bytes, 0, $bytes.Length)
$process.StandardInput.BaseStream.Flush()
$process.StandardInput.Close()
$second = $process.StandardOutput.ReadToEnd()
[void]$process.WaitForExit()
if ($second -ne $once.stdout) { throw 'formatter is not idempotent on the let-bound shapes' }

Write-Host 'STEP-0329 validator OK (matrix, e2e corpus, refusal corpus, nesting budget, formatter)'
