# STEP-0203: M22 S6 — Bool/Text constant IR and support-matrix correction

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0202
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, application support matrix, STEP-0083/report corrections, this record

## 1. Objective

Extend Rust-identical self-host lowering from `Int` constants/parameters
to `Bool` and `Text`, including the full accepted source escape set,
embedded NUL and Unicode, while retaining typed refusals and exact source
ranges.

## 2. Mechanism

- Function return signatures now map `Int`, `Bool`, and `Text` to IR
  kinds `int`, `bool`, and `string`.
- Single-token expressions classify canonical integers, `true`/`false`,
  quoted strings, or the one declared Int parameter. Expression and
  return types must match before any IR bytes are emitted.
- The lexer validates RFC-0006's six escapes while scanning the string;
  invalid escapes become a stable `ERR:E-SH-IR-STRING-ESCAPE` typed
  outcome.
- A valid Sico raw string token is already a valid JSON string token for
  `\"`, `\\`, `\n`, `\r`, and `\t`. `json_string_token` rewrites only
  Sico `\0` to canonical JSON `\u0000`, copying all other UTF-8 byte
  segments unchanged. This preserves Unicode without a second character
  decoder.
- IR operations are `const_bool` with a JSON boolean and `const_string`
  with the canonical JSON string payload; token byte spans include the
  source quotes, exactly matching Rust lowering.

## 3. Executable evidence

One real-built Sico frontend is run repeatedly on six positive sources:

1. Int constant;
2. Int parameter direct SSA return;
3. Bool `true`;
4. Text containing newline and escaped quotes;
5. Text containing source `\0`;
6. Unicode Text `你好`.

Every output deserializes as `sico_ir::Module`, passes the independent
verifier, round-trips canonically, and is byte-identical to Rust
`lower_core` for the same source and source name. Negative bare Int and
bad string escape inputs return typed `invalid-input` with no IR bytes.
The cumulative selfhost parser suite is 2/2 green on the real Windows x64
runner (the lowering test contains all eight positive/refusal probes).

## 4. Failed approaches and executable correction

- A loop around `sico.text.char_at` returning Text generated invalid Wasm
  (`values remaining on stack at end of block`). Replacing it with raw
  JSON-token preservation removes that lowering shape and is simpler.
- `sico.text.replace` was considered for `\0`, but real build returned
  `unsupported stdlib intrinsic`. Inspection confirms semantics/IR
  register it while codegen has no emission case. This is a pre-existing
  STEP-0083 documentation discrepancy, not a self-host limitation.

Per repository policy, the support claim is lowered in the same change:
STEP-0083 and its report no longer list `text.replace` as executable; the
machine matrix declares it check-accepted/build-refused and its validator
requires that entry. The original four pilot results remain valid because
none consumed `text.replace`.

## 5. Residuals

Multiple parameters/functions, calls, fixed-width constants/operations,
general blocks and complete token spans remain open. The invalid-Wasm
shape is avoided but not claimed as a backend fix; a future compiler
defect STEP may minimize and repair it independently.
