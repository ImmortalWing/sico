# STEP-0201: M22 S6 — first verifier-accepted, Rust-identical IR

> - status: complete
> - phase: M22 S6 typed-IR emission after STEP-0200
> - completed: 2026-09-17
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, runner dev dependency on `sico-source`, this STEP record

## 1. Objective

Cross the boundary from parser-owned text shapes to the repository's
actual typed IR contract. For one frozen scalar-return subset, the
Sico-written frontend must emit canonical IR bytes that deserialize as
`sico_ir::Module`, pass the independent Rust verifier, and equal the Rust
`lower_core` oracle byte-for-byte.

## 2. Frozen subset and behavior

`--emit-ir` accepts exactly one function with:

- zero parameters;
- return type `Int`;
- exactly one `return` whose expression is one canonical non-negative
  integer token;
- no let/set/control-flow statements.

It emits the full `sico.ir.v0` module/function/block/instruction/
terminator JSON shape with canonical IDs and source ranges. Function and
block ranges cover the declaration; the `const_int` instruction range is
the exact byte span of the literal. Bare negative Int stays refused,
matching the current Rust `lower_core` support boundary even though the
STEP-0200 parser tree can represent it.

All unsupported inputs return the standard Script
`InvalidInput` outcome with a stable `ERR:E-SH-IR-*` identity. They emit
no stdout/IR artifact.

## 3. Executable evidence

For `function answer() returns Int: return 7`, one test performs:

1. real `sico build --profile script-v0` of the Sico frontend;
2. real Wasmtime runner execution with `--emit-ir`;
3. `serde_json` deserialization into `sico_ir::Module`;
4. independent `sico_ir::verify` (zero errors);
5. canonical reserialization equality with the guest bytes;
6. Rust `lower_core` on the same source/name and byte-for-byte equality
   with the Sico-produced canonical JSON.

The same compiled guest receives `return -0` and returns typed
`invalid-input / ERR:E-SH-IR-NEGATIVE-BARE`; no IR output exists on that
path. Together with the STEP-0200 parser-tree test: 2/2 integration tests
green on Windows x64.

## 4. Findings corrected during the slice

- A source scan initially matched the `return` prefix of `returns`;
  token-boundary checking now prevents that and produces the exact Rust
  literal range (`40..41` in the frozen probe).
- Internal `Result[Text, ScriptError]` made the Script wrapper unable to
  prove its payload layout. Internal lowering now returns data-state
  `ERR:*`; only `main` converts it through a standard
  `Result[ScriptOutput, ScriptError]` helper, preserving the public typed
  boundary without a backend special case.
- Parallel integration tests previously shared a process-ID temp path;
  an atomic per-process suffix removes the collision.

## 5. Residuals

This proves one typed-IR slice, not full lowering or self-hosting. Next:
parameters/name resolution (with the exact operation shape decided by
the Rust differential), Bool/String constants, recursive
call arguments and fixed-width operations, typed source spans for all
tokens, then cumulative corpus comparison. Codegen, RFC-0011 Component
byte equality, and ADR-0015 `A == B == C` remain open.

### Correction from STEP-0202

Executable Rust lowering shows that `return value` for an `Int` parameter
emits no `Copy`; the terminator references parameter `ValueId(0)`
directly. STEP-0202 records and matches that SSA shape. Any earlier
forward-looking shorthand that named `Copy` is not a support contract.
