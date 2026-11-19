<#
.SYNOPSIS
Language v1 batch 4 frontier probe (STEP-0629).

.DESCRIPTION
Measures the production (Rust oracle) frontier for the enum-payload-match and
Option shapes that RFC-0056 (language v1 batch 4) will land. Every probe is a
self-contained Script-profile source that the toolchain must either build or
refuse; the frozen expected table below records the 2026-11-19 measurement.

This probe intentionally exercises the RUST frontend/codegen, not the M22 guest
frontend (tools/probe-m22-frontend-frontier.ps1 covers that surface).

Probe set:
  p1 payload construct at a function-result boundary
  p2 wildcard match arm with an enum-typed parameter
  p3 single-payload binding match arm        (G1 sentinel)
  p4 multi-payload binding match arm          (G1 sentinel, arity 2)
  p4b nested (non-binding) pattern match arm  (semantics cross-product)
  p5 Option `some(...)` constructor           (G2 absence, STEP-0494 fact-fix)
  p6 Result ok(h) payload binding anchor      (control: accepted today)
  p7 transient inline construct + wildcard match (enum value never crosses a
     function boundary)

Exit codes: 0 = frontier unchanged (all probes match the frozen table),
1 = frontier moved or a probe produced an unexpected outcome.

Usage:
  pwsh -File tools/probe-language-v1-batch4-frontier.ps1
  pwsh -File tools/probe-language-v1-batch4-frontier.ps1 -Json
#>
[CmdletBinding()]
param(
    [switch]$Json
)

$ErrorActionPreference = 'Continue'

$repoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$cli = Join-Path $repoRoot 'target\debug\sico.exe'
$workDir = Join-Path $repoRoot 'target\tmp\step0629'

if (-not (Test-Path $cli)) {
    Write-Error "sico CLI not found at $cli (build with: cargo build -p sico-cli)"
    exit 1
}

New-Item -ItemType Directory -Force -Path $workDir | Out-Null

# The frozen Script ABI preamble every Script-profile probe shares.
$preamble = @'
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

'@

# Entry point required by the script-v0 profile.
$entry = @'
function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))
end function
'@

# Each case: name, probe body (declarations + functions), expected verdict.
$scriptEntryFlag = '--profile'
$scriptEntryValue = 'script-v0'
$cases = @(
    @{
        Name = 'p1-payload-construct'
        Body = @'
enum Notice:
  case Message(text: Text)
  case Stop
end enum

function mk() returns Notice:
  return Notice.Message("hi")
end function

'@
        Expected = 'refused:codegen:non-scalar-result'
    },
    @{
        Name = 'p2-wildcard-arm'
        Body = @'
enum Notice:
  case Message(text: Text)
  case Stop
end enum

function is_message(notice: Notice) returns Bool:
  match notice:
    case Notice.Message(_):
      return true
    case Notice.Stop:
      return false
  end match
end function

'@
        Expected = 'refused:codegen:non-scalar-parameter'
    },
    @{
        Name = 'p3-binding-arm'
        Body = @'
enum Notice:
  case Message(text: Text)
  case Stop
end enum

function text_of(notice: Notice) returns Text:
  match notice:
    case Notice.Message(t):
      return t
    case Notice.Stop:
      return "stopped"
  end match
end function

'@
        # 2026-11-19 G1 landed: the arm-level sentinel is gone. These probes
        # declare enum-typed function SIGNATURES, which stay refused at the
        # boundary in v0 by design (RFC-0056 non-goal); transient shapes
        # (p8/p10/p11) now build and run.
        Expected = 'refused:codegen:non-scalar-parameter'
    },
    @{
        Name = 'p4-multi-payload-binding'
        Body = @'
enum PairBox:
  case Pair(left: U64, right: U64)
  case Empty
end enum

function first(box: PairBox) returns U64:
  match box:
    case PairBox.Pair(a, b):
      return a
    case PairBox.Empty:
      return U64.literal(0)
  end match
end function

'@
        Expected = 'refused:codegen:non-scalar-parameter'
    },
    @{
        Name = 'p4b-nested-pattern-arm'
        Body = @'
enum Notice:
  case Message(text: Text)
  case Stop
end enum

enum Wrap:
  case Box(inner: Notice)
  case Nothing
end enum

function is_stop(wrap: Wrap) returns Bool:
  match wrap:
    case Wrap.Box(Notice.Stop):
      return true
    case Wrap.Box(_):
      return false
    case Wrap.Nothing:
      return false
  end match
end function

'@
        Expected = 'refused:semantics:E3001-exhaustiveness'
    },
    @{
        Name = 'p5-option-some-construct'
        Body = @'
function probe() returns Option[I64]:
  return some(I64.literal(1))
end function

'@
        # 2026-11-19 G2 landed: `some` infers Option (the E2031 identity is
        # gone); this probe declares an Option-typed function signature,
        # which the Script v0 boundary refuses by design (RFC-0056 non-goal)
        # — the same boundary rule as user enums.
        Expected = 'refused:codegen:non-scalar-result'
    },
    @{
        Name = 'p6-result-binding-anchor'
        Body = @'
function half(value: U64) returns U64:
  match U64.checked_div(value, U64.literal(2)):
    case ok(h):
      return h
    case error(_):
      return U64.literal(0)
  end match
end function

'@
        Expected = 'accepted'
    },
    @{
        Name = 'p8-inline-binding'
        FullSource = $true
        Body = @'
enum Notice:
  case Message(text: Text)
  case Stop
end enum

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  match Notice.Message("x"):
    case Notice.Message(t):
      return ok(ScriptOutput(stdout: sico.text.encode(t), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
    case Notice.Stop:
      return ok(ScriptOutput(stdout: input.stdin, stderr: sico.text.encode(""), exit_code: I64.literal(1)))
  end match
end function
'@
        Expected = 'accepted'
    },
    @{
        Name = 'p10-inline-multi-payload'
        FullSource = $true
        Body = @'
enum PairBox:
  case Pair(left: U64, right: U64)
  case Empty
end enum

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  match PairBox.Pair(U64.literal(7), U64.literal(9)):
    case PairBox.Pair(a, b):
      return ok(ScriptOutput(stdout: sico.text.encode(sico.u64.to_text(a)), stderr: sico.text.encode(sico.u64.to_text(b)), exit_code: I64.literal(0)))
    case PairBox.Empty:
      return ok(ScriptOutput(stdout: input.stdin, stderr: sico.text.encode(""), exit_code: I64.literal(1)))
  end match
end function
'@
        Expected = 'accepted'
    },
    @{
        Name = 'p11-let-bound-enum'
        FullSource = $true
        Body = @'
enum Notice:
  case Message(text: Text)
  case Stop
end enum

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  let x = match Notice.Message("hello"):
    case Notice.Message(t):
      t
    case Notice.Stop:
      "stopped"
  end match
  return ok(ScriptOutput(stdout: sico.text.encode(x), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
end function
'@
        Expected = 'accepted'
    },
    @{
        Name = 'p13-typed-poll-event'
        # RFC-0056 D5/D7: the pinned-shape host enum declaration plus the
        # typed poll_event match (build-level evidence; the runtime path
        # is pinned by the runner's ui_source suite).
        FullSource = $true
        Body = @'
enum ui_event:
  case click(id: U64)
  case select(handle: U64, index: U64)
  case closed
end enum

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  match sico.ui.poll_event():
    case some(ev):
      match ev:
        case ui_event.click(h):
          return ok(ScriptOutput(stdout: sico.text.encode(sico.u64.to_text(h)), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
        case ui_event.select(h, i):
          return ok(ScriptOutput(stdout: sico.text.encode(sico.u64.to_text(h)), stderr: sico.text.encode(sico.u64.to_text(i)), exit_code: I64.literal(0)))
        case ui_event.closed:
          return ok(ScriptOutput(stdout: sico.text.encode("closed"), stderr: sico.text.encode(""), exit_code: I64.literal(0)))
      end match
    case none:
      return ok(ScriptOutput(stdout: input.stdin, stderr: sico.text.encode(""), exit_code: I64.literal(1)))
  end match
end function
'@
        Expected = 'accepted'
    },
    @{
        Name = 'p14-ui-event-shape-mismatch'
        # RFC-0056 D5: a wrong-shape declaration is the typed E2028 refusal.
        FullSource = $true
        Body = @'
enum ui_event:
  case click(id: U64)
  case closed
end enum

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  match sico.ui.poll_event():
    case some(ev):
      return ok(ScriptOutput(stdout: input.stdin, stderr: sico.text.encode(""), exit_code: I64.literal(0)))
    case none:
      return ok(ScriptOutput(stdout: input.stdin, stderr: sico.text.encode(""), exit_code: I64.literal(1)))
  end match
end function
'@
        Expected = 'refused:semantics:E2028-shape'
    },
    @{
        Name = 'p7-inline-transient'
        # No preamble: this probe declares its own Notice and matches an
        # inline construct; the enum value never crosses a function boundary.
        FullSource = $true
        Body = @'
enum Notice:
  case Message(text: Text)
  case Stop
end enum

function main(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:
  match Notice.Message("x"):
    case Notice.Message(_):
      return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))
    case Notice.Stop:
      return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(1)))
  end match
end function
'@
        Expected = 'accepted'
    }
)

function Measure-Verdict($sourcePath, $wasmPath) {
    # Capture via cmd redirection files: PowerShell 5.1's native-stderr
    # handling wraps ErrorRecords at console width and can split a refusal
    # phrase mid-word, which corrupts identity matching.
    $outFile = "$sourcePath.stdout.txt"
    $errFile = "$sourcePath.stderr.txt"
    & cmd /c "`"$cli`" build $scriptEntryFlag $scriptEntryValue `"$sourcePath`" -o `"$wasmPath`" >`"$outFile`" 2>`"$errFile`"" | Out-Null
    $exitCode = $LASTEXITCODE
    $text = [System.IO.File]::ReadAllText($outFile) + ' ' + [System.IO.File]::ReadAllText($errFile)
    $flat = ($text -replace "`r?`n", ' ').Trim()
    if ($exitCode -eq 0) {
        return @{ Verdict = 'accepted'; Detail = '' }
    }
    if ($flat -match 'unsupported match payload binding') {
        return @{ Verdict = 'refused:lower:match-payload-binding'; Detail = $flat }
    }
    if ($flat -match 'unsupported non-scalar result') {
        return @{ Verdict = 'refused:codegen:non-scalar-result'; Detail = $flat }
    }
    if ($flat -match 'unsupported non-scalar parameter') {
        return @{ Verdict = 'refused:codegen:non-scalar-parameter'; Detail = $flat }
    }
    if ($flat -match 'E3001') {
        return @{ Verdict = 'refused:semantics:E3001-exhaustiveness'; Detail = $flat }
    }
    if ($flat -match 'E2031') {
        return @{ Verdict = 'refused:semantics:E2031-unresolved-call'; Detail = $flat }
    }
    if ($flat -match 'E2028') {
        return @{ Verdict = 'refused:semantics:E2028-shape'; Detail = $flat }
    }
    return @{ Verdict = 'failed'; Detail = $flat.Substring(0, [Math]::Min(120, $flat.Length)) }
}

$results = @()
foreach ($case in $cases) {
    if ($case.FullSource) {
        $source = $preamble + $case.Body
    } else {
        $source = $preamble + $case.Body + $entry
    }
    $path = Join-Path $workDir ($case.Name + '.sico')
    [System.IO.File]::WriteAllText($path, ($source -replace "`r`n", "`n"))
    $wasmPath = Join-Path $workDir ($case.Name + '.wasm')
    if (Test-Path $wasmPath) { Remove-Item $wasmPath -Force }
    $measured = Measure-Verdict $path $wasmPath
    $built = Test-Path $wasmPath
    $results += [pscustomobject]@{
        Name = $case.Name
        Expected = $case.Expected
        Actual = $measured.Verdict
        Detail = $measured.Detail
        ArtifactBytes = if ($built) { (Get-Item $wasmPath).Length } else { 0 }
        Match = ($case.Expected -eq $measured.Verdict)
    }
}

if ($Json) {
    $results | ConvertTo-Json -Depth 4
} else {
    foreach ($result in $results) {
        $status = if ($result.Match) { 'OK  ' } else { 'MOVED' }
        Write-Output ("{0} {1,-28} expected={2,-42} actual={3}" -f $status, $result.Name, $result.Expected, $result.Actual)
    }
    $moved = @($results | Where-Object { -not $_.Match })
    if ($moved.Count -eq 0) {
        Write-Output 'FRONTIER_UNCHANGED'
        exit 0
    }
    Write-Output ('FRONTIER_MOVED count=' + $moved.Count)
    exit 1
}
