<#
.SYNOPSIS
M22 S5 frontend frontier probe (STEP-0488).

.DESCRIPTION
Probes the guest frontend (selfhost/compiler.sico) with the Script-profile
shapes that the IR-driven component emitter needs. Every probe is a source
that the guest must either lower byte-exactly against the Rust oracle or
refuse with a typed identity (no silent fallback, ADR-0016 decision 6).

The probe reports the current frontier: shapes the guest lowers are "LOWERED",
shapes the guest refuses with a typed refusal are "REFUSED <code>", and any
other outcome is a FAILURE for this step.

Exit codes: 0 = frontier unchanged (all probes match the recorded table),
1 = frontier moved (a probe changed verdict) or a probe produced an
unexpected outcome.

Usage:
  pwsh -File tools/probe-m22-frontend-frontier.ps1
  pwsh -File tools/probe-m22-frontend-frontier.ps1 -Json
#>
[CmdletBinding()]
param(
    [switch]$Json
)

$ErrorActionPreference = 'Continue'

$repoRoot = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
$cli = Join-Path $repoRoot 'target\debug\sico.exe'
$runner = Join-Path $repoRoot 'runner\sico-runner\target\debug\sico-runner.exe'
$workDir = Join-Path $repoRoot 'target\tmp\frontend-frontier'

if (-not (Test-Path $cli)) {
    Write-Error "sico CLI not found at $cli (build with: cargo build -p sico-cli)"
    exit 1
}
if (-not (Test-Path $runner)) {
    Write-Error "sico-runner not found at $runner (build with: cargo build --manifest-path runner/sico-runner/Cargo.toml --bins)"
    exit 1
}

$env:SICO_RUNNER = $runner
New-Item -ItemType Directory -Force -Path $workDir | Out-Null

# Each case: name, source, expected verdict ('lowered' | 'refused:<CODE>').
$recordDecl = "record ScriptInput:`n  field stdin: Bytes`nend record`n"
$cases = @(
    @{
        Name = 'scalar-identity'
        Source = "function probe(value: I64) returns I64:`n  return value`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'record-declaration'
        Source = "function probe(value: I64) returns I64:`n  return value`nend function`n`n" + $recordDecl
        Expected = 'lowered'
    }
    @{
        Name = 'record-parameter-projection'
        Source = $recordDecl + "function probe(input: ScriptInput) returns Bytes:`n  return input.stdin`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'record-parameter-projection-missing-record'
        Source = "function probe(input: ScriptInput) returns Bytes:`n  return input.stdin`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-EXPRESSION'
    }
    @{
        Name = 'record-parameter-projection-unknown-field'
        Source = $recordDecl + "function probe(input: ScriptInput) returns Bytes:`n  return input.missing`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-EXPRESSION'
    }
    @{
        Name = 'record-parameter-projection-generic-record'
        Source = "record ScriptOutput:`n  field stdout: Bytes`nend record`n`nfunction probe(out: ScriptOutput) returns Bytes:`n  return out.stdout`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'named-record-return-type'
        Source = "record ScriptOutput:`n  field stdout: Bytes`n  field stderr: Bytes`nend record`n`nfunction probe(out: ScriptOutput) returns ScriptOutput:`n  return out`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'named-record-return-type-undeclared'
        Source = "function probe(out: ScriptOutput) returns ScriptOutput:`n  return out`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-PARAMETER-TYPE'
    }
    @{
        Name = 'named-record-return-type-statements'
        Source = "record ScriptOutput:`n  field stdout: Bytes`n  field stderr: Bytes`nend record`n`nfunction probe(out: ScriptOutput) returns ScriptOutput:`n  let x = U64.literal(0)`n  return out`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-RETURN-TYPE'
    }
    @{
        Name = 'record-construct'
        Source = "record ScriptInput:`n  field arguments: List[Text]`n  field stdin: Bytes`nend record`n`nrecord ScriptOutput:`n  field stdout: Bytes`n  field stderr: Bytes`n  field exit_code: I64`nend record`n`nfunction probe(input: ScriptInput) returns ScriptOutput:`n  return ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0))`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'record-construct-unknown-field'
        Source = "record ScriptInput:`n  field arguments: List[Text]`n  field stdin: Bytes`nend record`n`nrecord ScriptOutput:`n  field stdout: Bytes`n  field stderr: Bytes`n  field exit_code: I64`nend record`n`nfunction probe(input: ScriptInput) returns ScriptOutput:`n  return ScriptOutput(missing: input.stdin)`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-CONSTRUCT-FIELD'
    }
    @{
        Name = 'record-construct-unsupported-value'
        Source = "record ScriptInput:`n  field arguments: List[Text]`n  field stdin: Bytes`nend record`n`nrecord ScriptOutput:`n  field stdout: Bytes`n  field stderr: Bytes`n  field exit_code: I64`nend record`n`nfunction probe(input: ScriptInput) returns ScriptOutput:`n  return ScriptOutput(stdout: sico.text.encode(""x""))`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-CONSTRUCT-VALUE'
    }
    @{
        Name = 'result-variant-return'
        Source = "function probe() returns Result[I64, NumericError]:`n  return ok(I64.literal(1))`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'result-variant-payload-type-mismatch'
        Source = "function probe() returns Result[I64, NumericError]:`n  return ok(""text"")`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-VARIANT-TYPE'
    }
    @{
        Name = 'result-variant-unsupported-payload'
        Source = "function probe() returns Result[I64, NumericError]:`n  return ok(sico.text.encode(""x""))`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-VARIANT-PAYLOAD'
    }
    @{
        Name = 'result-variant-undeclared-result-type'
        Source = "function probe() returns I64:`n  return ok(I64.literal(1))`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-CALL-TARGET'
    }
    @{
        Name = 'option-variant-return'
        Source = "function probe() returns Option[I64]:`n  return some(I64.literal(1))`nend function`n"
        Expected = 'refused:ERR:E-SH-IR-CALL-TARGET'
    }
    @{
        Name = 'user-call-in-return-position'
        Source = "function helper(value: I64) returns I64:`n  return value`nend function`n`nfunction probe() returns I64:`n  return helper(I64.literal(3))`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'user-call-in-let-binding'
        Source = "function helper(value: I64) returns I64:`n  return value`nend function`n`nfunction probe() returns I64:`n  let x = helper(I64.literal(3))`n  return x`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'host-call-in-return-position'
        Source = "function probe() returns U64:`n  return sico.text.length(""abc"")`nend function`n"
        # STEP-0529: the T2 frontend knife claims the `sico.text.length` statement
        # positions, so this shape (a call with a string literal in a return
        # position) now lowers.  Byte-exactness against the Rust oracle is proven
        # by the firmware pin
        # `sico_compiler_lowers_the_text_length_in_a_statement_position_byte_exactly`
        # (the equivalent `return sico.text.length("ab")` fixture, 2252 B).
        Expected = 'lowered'
    }
    @{
        Name = 'host-call-in-straight-line-body'
        Source = "function size(data: Bytes) returns U64:`n  return sico.bytes.length(data)`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'host-call-in-let-binding'
        Source = "function size(data: Bytes) returns U64:`n  let n = sico.bytes.length(data)`n  return n`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-in-let-binding'
        Source = "function join(a: Text, b: Text) returns Text:`n  let c = sico.text.concat(a, b)`n  return c`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-in-return-position'
        Source = "function join(a: Text, b: Text) returns Text:`n  return sico.text.concat(a, b)`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-with-literal-operands'
        Source = "function join() returns Text:`n  return sico.text.concat(""a"", ""b"")`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-with-a-mixed-operand-pair'
        Source = "function join(a: Text) returns Text:`n  return sico.text.concat(a, ""\t"")`nend function`n"
        Expected = 'lowered'
    }
    # STEP-0523: the single-`return` route used to accept two `Text`
    # parameters only, so every other operand shape was refused even though
    # the Rust backend lowers it. The route now shares the operand lowering
    # with the `let`-binding routes, and these cases pin the widened frontier.
    @{
        Name = 'text-concat-with-a-user-call-operand'
        Source = "function join(slot: U64) returns Text:`n  return sico.text.concat(""20"", helper(slot))`nend function`n`nfunction helper(x: U64) returns Text:`n  return ""z""`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-with-a-user-call-operand-first'
        Source = "function join(slot: U64) returns Text:`n  return sico.text.concat(helper(slot), ""20"")`nend function`n`nfunction helper(x: U64) returns Text:`n  return ""z""`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-with-a-nested-user-call-operand'
        Source = "function join(slot: U64) returns Text:`n  return sico.text.concat(""20"", sico.text.concat(helper(slot), ""x""))`nend function`n`nfunction helper(x: U64) returns Text:`n  return ""z""`nend function`n"
        Expected = 'lowered'
    }
    # A user call with **two** arguments is now covered: the operand helper
    # lowers a call operand with several arguments.
    @{
        Name = 'text-concat-with-a-two-argument-user-call-operand'
        Source = "function join(a: Text, b: U64) returns Text:`n  return sico.text.concat(helper(a, b), ""20"")`nend function`n`nfunction helper(x: Text, y: U64) returns Text:`n  return ""z""`nend function`n"
        Expected = 'lowered'
    }
    # STEP-0524: the shape that is really in S -- a call operand whose second
    # argument is a `U64.literal(n)` constant, beside a nested concat.
    @{
        Name = 'text-concat-with-a-u64-literal-argument-call-operand'
        Source = "function join(items: List[Text]) returns Text:`n  return sico.text.concat(at(items, U64.literal(1)), sico.text.concat(""."", at(items, U64.literal(2))))`nend function`n`nfunction at(items: List[Text], index: U64) returns Text:`n  return ""z""`nend function`n"
        Expected = 'lowered'
    }
    # STEP-0525: the shape the G-f2 census found in S -- a call operand whose
    # argument is itself a call.  The operand lowerer recurses into the nested
    # call, so the value ids compose.
    @{
        Name = 'text-concat-with-a-nested-call-argument-operand'
        Source = "function join(a: Text) returns Text:`n  return sico.text.concat(helper(inner(a), ""b""), ""20"")`nend function`n`nfunction helper(x: Text, y: Text) returns Text:`n  return ""z""`nend function`n`nfunction inner(v: Text) returns Text:`n  return ""w""`nend function`n"
        Expected = 'lowered'
    }
    # The nested call without any argument of its own -- 9 of the 22 S sites.
    @{
        Name = 'text-concat-with-a-zero-argument-nested-call-operand'
        Source = "function join(a: Text) returns Text:`n  return sico.text.concat(helper(zero(), ""b""), ""20"")`nend function`n`nfunction helper(x: Text, y: Text) returns Text:`n  return ""z""`nend function`n`nfunction zero() returns Text:`n  return ""w""`nend function`n"
        Expected = 'lowered'
    }
    # The frontend lowers a concat inside an `if` block; the **component
    # emission** refuses it, because the guest's emitter only knows a single
    # straight-line block. This probe measures the frontend, so the verdict is
    # `lowered` -- the emitter gap is registered in the T1 support surface.
    @{
        Name = 'text-concat-inside-an-if-block'
        Source = "function join(a: Text) returns Text:`n  if U64.equal(U64.literal(1), U64.literal(1)):`n    return sico.text.concat(""a"", a)`n  end if`n  return """"`nend function`n"
        Expected = 'lowered'
    }
    # STEP-0521: the same operand kinds lower byte for byte once the body
    # carries one more statement, so the refusal above is a route gate, not a
    # missing capability. These three cases pin that measured frontier.
    @{
        Name = 'text-concat-with-a-mixed-operand-pair-and-a-statement'
        Source = "function join(a: Text) returns Text:`n  let x = a`n  return sico.text.concat(a, ""x"")`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-with-a-nested-host-call-and-a-statement'
        Source = "function join(a: Text, b: Text) returns Text:`n  let x = a`n  return sico.text.concat(a, sico.text.concat(b, a))`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-with-an-escaped-literal-operand'
        Source = "function join(a: Text) returns Text:`n  let x = a`n  return sico.text.concat(a, ""\t"")`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-with-a-nul-escaped-literal-operand'
        Source = "function join(a: Text) returns Text:`n  let x = a`n  return sico.text.concat(a, ""\0"")`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-with-an-empty-literal-operand'
        Source = "function join(a: Text) returns Text:`n  let x = a`n  return sico.text.concat(a, """")`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-concat-in-return-position-with-a-statement'
        Source = "function join(a: Text, b: Text) returns Text:`n  let x = a`n  return sico.text.concat(a, b)`nend function`n"
        Expected = 'lowered'
    }
    # STEP-0528: the T2 family (`sico.text.length`).  The measurement split the
    # family in two: the condition positions lower byte for byte, the
    # statement positions stayed typed refusals.  STEP-0529 flipped the
    # statement positions and STEP-0532 flipped the nested-call-argument
    # shape, so every case below lowers now.
    @{
        Name = 'text-length-in-an-if-condition'
        Source = "function size(t: Text) returns U64:`n  if U64.equal(sico.text.length(t), U64.literal(2)):`n    return U64.literal(1)`n  end if`n  return U64.literal(0)`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-in-a-while-condition'
        Source = "function size(t: Text) returns U64:`n  let i = U64.literal(0)`n  while U64.less_than(i, sico.text.length(t)):`n    set i = U64.literal(1)`n  end while`n  return i`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-inside-an-if-block'
        Source = "function size(t: Text) returns U64:`n  let n = U64.literal(0)`n  if U64.equal(U64.literal(1), U64.literal(1)):`n    set n = sico.text.length(t)`n  end if`n  return n`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-in-a-let-binding'
        Source = "function size(t: Text) returns U64:`n  let n = sico.text.length(t)`n  return n`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-in-a-set-position'
        Source = "function size(t: Text) returns U64:`n  let n = U64.literal(0)`n  set n = sico.text.length(t)`n  return n`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-in-a-return-position'
        Source = "function size(t: Text) returns U64:`n  return sico.text.length(t)`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-with-a-literal-argument'
        Source = "function size() returns U64:`n  return sico.text.length(`"ab`")`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-over-a-local-binding'
        Source = "function size(t: Text) returns U64:`n  let x = t`n  let n = sico.text.length(x)`n  return n`nend function`n"
        Expected = 'lowered'
    }
    # STEP-0532: the T2-g2 implementation knife moved the **frontend** on the
    # nested-call-argument shape -- the argument of `sico.text.length` may now
    # be a call itself.  All four cases below lower; the **component** layer
    # of two of them is still refused by the emitter, because the inner
    # operator is `sico.bytes.utf8_decode` (family T11), not because of this
    # family.
    @{
        Name = 'text-length-over-a-nested-call-argument'
        Source = "function size(src: Bytes) returns U64:`n  let n = sico.text.length(sico.bytes.utf8_decode(src))`n  return n`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-over-a-nested-call-argument-in-a-return-position'
        Source = "function size(t: Text) returns U64:`n  return sico.text.length(sico.text.concat(t, `"x`"))`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-over-a-nested-user-call-argument'
        Source = "function size(t: Text) returns U64:`n  return sico.text.length(helper(t))`nend function`nfunction helper(v: Text) returns Text:`n  return v`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'text-length-over-a-nested-host-call-argument-in-a-let-binding'
        Source = "function size(src: Bytes) returns U64:`n  let n = sico.text.length(sico.bytes.utf8_decode(src))`n  return n`nend function`n"
        Expected = 'lowered'
    }
    # STEP-0532: the shape the knife added -- the nested call in a `set`
    # position -- is pinned here too, so the frontier keeps the shape the
    # family declares.
    @{
        Name = 'text-length-over-a-nested-call-argument-in-a-set-position'
        Source = "function size(t: Text) returns U64:`n  let n = U64.literal(0)`n  set n = sico.text.length(sico.text.concat(t, `"x`"))`n  return n`nend function`n"
        Expected = 'lowered'
    }
    @{
        Name = 'script-profile-run'
        Source = "record ScriptInput:`n  field arguments: List[Text]`n  field stdin: Bytes`nend record`n`nrecord ScriptOutput:`n  field stdout: Bytes`n  field stderr: Bytes`n  field exit_code: I64`nend record`n`nenum ScriptErrorCode:`n  case InvalidInput`nend enum`n`nrecord ScriptError:`n  field code: ScriptErrorCode`n  field message: Text`nend record`n`nfunction run(input: ScriptInput) returns Result[ScriptOutput, ScriptError]:`n  return ok(ScriptOutput(stdout: input.stdin, stderr: input.stdin, exit_code: I64.literal(0)))`nend function`n"
        Expected = 'lowered'
    }
)

$results = @()
foreach ($case in $cases) {
    $path = Join-Path $workDir ($case.Name + '.sico')
    [System.IO.File]::WriteAllText($path, ($case.Source -replace "`r`n", "`n"))
    $raw = Get-Content $path -Raw | & $cli run (Join-Path $repoRoot 'selfhost\compiler.sico') 2>&1 | Out-String
    $flat = ($raw -replace "`r?`n", " ")
    $marker = $flat.IndexOf('ERR:')
    if ($marker -ge 0) {
        $code = $flat.Substring($marker)
        $end = $code.IndexOf('"')
        if ($end -gt 0) { $code = $code.Substring(0, $end) }
        $code = $code.TrimEnd('\', '"', ' ')
        $verdict = 'refused:' + $code
    } elseif ($flat -match '"schema":"sico\.ir\.v0"') {
        $verdict = 'lowered'
    } else {
        $verdict = 'failed:' + $flat.Substring(0, [Math]::Min(80, $flat.Length))
    }
    $results += [pscustomobject]@{
        Name = $case.Name
        Expected = $case.Expected
        Actual = $verdict
        Match = ($case.Expected -eq $verdict)
    }
}

if ($Json) {
    $results | ConvertTo-Json -Depth 4
} else {
    foreach ($result in $results) {
        $status = if ($result.Match) { 'OK  ' } else { 'MOVED' }
        Write-Output ("{0} {1,-30} expected={2,-34} actual={3}" -f $status, $result.Name, $result.Expected, $result.Actual)
    }
    $moved = @($results | Where-Object { -not $_.Match })
    if ($moved.Count -eq 0) {
        Write-Output 'FRONTIER_UNCHANGED'
        exit 0
    }
    Write-Output ('FRONTIER_MOVED count=' + $moved.Count)
    exit 1
}
