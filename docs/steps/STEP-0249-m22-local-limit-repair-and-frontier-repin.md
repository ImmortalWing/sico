# STEP-0249: M22 local-limit repair and canary frontier re-pin

> - status: complete / partial-S4; M22 NO-GO
> - phase: M22 S4 canonical control-flow convergence
> - completed: 2026-09-21
> - evidence: internal-fixture, Windows x64 GNU real runner, PowerShell 5.1 host

## Objective

Restore a buildable, verifiable self-host tree and re-pin the typed canary
frontier with executed evidence.  The uncommitted STEP-0246–0248 state could
not be reproduced on this host: the M22 validators used PowerShell 7-only
APIs, and the accumulated `parser.sico` growth pushed
`control_while_function_ir` past the frozen 256-local IR verifier limit, so
the self-host components failed to lower at all
(`IR verifier rejected compiler output: function[75]: Limit`).

## Changes

- Extracted the `sico.bytes.at` while-body RHS serializer from
  `control_while_function_ir` into a reusable `while_bytes_at_rhs_packed`
  helper with the same 12-parameter shape and `\n`-packed return protocol as
  the existing `while_call_rhs_packed`.  The overflow function dropped from
  >256 locals to 238; behavior is byte-identical (7/7 differential suite).
- Added `runner/sico-runner/tests/selfhost_local_bounds.rs`: every function
  of the assembled parser link unit must stay inside the frozen verifier
  bounds, with 16 locals of headroom on the busiest function.
- Made the M22 validators and corpus updater runnable on Windows PowerShell
  5.1 without changing semantics: SHA-256 via `SHA256.Create().ComputeHash`,
  byte-loop reproducibility compare, `.Arguments` quoting instead of
  `ArgumentList`, raw UTF-8 stdin through the pipe base stream (the console
  codepage corrupted a U+2014 in `infix-equality.sico` and produced a false
  `format_idempotent=false`), ordinal path sort, and a deterministic
  2-space JSON serializer so the frozen manifests stay byte-identical across
  PowerShell 5.1 and 7 hosts.
- Re-froze `selfhost/corpus-v0.json` and `selfhost/script-build-corpus-v0.json`
  from the current sources: the `block-solver.sico` entry now records the
  canonical LF bytes (47898, sha 08a11345…) instead of the CRLF
  materialization — the P0-5 closure is now actually executed — and six
  component hashes were re-recorded against the current compiler (the
  working-tree stdlib linear-builder change); in-run double-build
  reproducibility checks passed for all 215 entries.
- Re-measured the formatter canary frontier: after the repair the full
  formatter fails typed at `ERR:E-SH-IR-CALL-TYPE` (previously recorded as
  `E-SH-IR-EXPRESSION`), because while-body `sico.bytes.at` lets now lower
  and the first refusal moved into the while-body if-condition operand
  machinery, which does not yet accept fixed-width literal operands such as
  `I64.literal(32)` and rejects Bytes-parameter functions in
  `scalar_type_kind`.  `validate-step-0247/0248.ps1` now pin the measured
  frontier; this STEP records the move.

## Executed validation

- `validate-step-0246.ps1`, `validate-step-0247.ps1`, `validate-step-0248.ps1`
  green on PowerShell 5.1 (corpus 215/215, unified lowering, byte-parity
  suites, typed canary);
- `selfhost_compiler` 7/7, `selfhost_checker` 8/8, `selfhost_parser` 2/2,
  `list_append_cow`, `bootstrap_bundle`, and the new `selfhost_local_bounds`
  through the real Script runner;
- minimal-probe evidence: while conditions with intrinsic calls and cell
  arguments lower (`U64.less_than(c, b)`), while-body user-call lets and sets
  lower, straight-line `let b = sico.bytes.at(...)` and while-body
  `I64.literal(n)` if-operands remain the typed refusal frontier;
- `git diff --check` passes.

## Honest residuals

- The canary boundary is `E-SH-IR-CALL-TYPE` at the while-body if-condition
  operand machinery; `scalar_type_kind` still refuses `Bytes` parameters on
  the while path.  Passing the seven-function prefix is not a complete
  formatter canary.
- S5 general deterministic Component codegen, S6 A=B=C/bootstrap packaging and
  budget gates, and S7 exit audit remain open.  M22 therefore remains NO-GO.
- The STEP-0246–0248 validation claims could not be reproduced on this host
  before this STEP (PS7-only validators, then the local-limit build failure);
  the green state recorded here is the first executed on-host evidence.
