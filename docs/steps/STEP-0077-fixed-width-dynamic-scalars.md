# STEP-0077: Fixed-width dynamic scalar representation

> - status: complete
> - phase: M8
> - started: 2026-07-17
> - completed: 2026-07-17
> - owners: autonomous-agent

## 1. Objective

Add explicit `I64` and `U64` representations from semantic analysis through verified Sico IR, Core Wasm and Component exports so non-constant parameters, locals, checked arithmetic and comparisons execute without weakening arbitrary-precision `Int`.

## 2. Context and evidence

STEP-0076 selected versioned Program/Adapter composition. The current IR only has `Type::Int` plus `ConstInt`/`AddInt`; the backend accepts `Int` results only when compile-time proven to fit `i64`, rejects `Int` parameters, and rejects non-constant `Int` copies/addition. RFC-0029 explicitly freezes `I64/U64` as separate Script machine values and forbids silently reinterpreting `Int`.

## 3. Scope

Included:

- semantic/prelude identities for `I64` and `U64` without implicit conversion from `Int`;
- typed IR scalar variants and explicit checked arithmetic/comparison operations;
- verifier operand/result rules;
- dynamic parameter/local/result Core Wasm and scalar Component lowering;
- exact overflow/underflow behavior and deterministic oracle/property tests;
- focused source-to-Wasmtime fixtures where the current frontend can express the operations.

Excluded:

- general `Call` and multi-block codegen (STEP-0078);
- Text/Bytes/List/record/Result Canonical ABI (STEP-0079);
- general arbitrary-precision `Int` Runtime representation;
- adapter packaging, manifest v1, runner and CLI integration.

## 4. Options and decision

The frozen direction is distinct fixed-width types, not backend-only narrowing of `Int`. Source uses `I64.literal`/`U64.literal` for checked compile-time literal construction and type-directed `checked_add`, `checked_sub`, `equal` and `less_than` intrinsics. Checked arithmetic returns `Result[I64|U64, NumericError]`, where `NumericError` has `overflow` and `underflow`; ordinary `+` on fixed-width values is rejected instead of wrapping or trapping.

IR uses the smaller generic operation set `CheckedAdd`, `CheckedSub`, `EqualFixed` and `LessFixed`; verified operand types select signed or unsigned behavior. Core Wasm exposes checked results as `(tag, payload)` multi-values. Component lifting uses the Canonical ABI indirect-result convention through a private return area, preserving a real Component `result` value rather than degrading failure to a trap.

## 5. Plan

1. Audit semantic builtin/type handling, integer-literal typing and existing no-implicit-conversion rules.
2. Freeze IR JSON names and checked arithmetic/comparison operation set.
3. Add verifier mutations for signedness, operand/result mismatch and illegal `Int` narrowing.
4. Implement dynamic Wasm locals/parameters/results and checked overflow/underflow paths.
5. Add boundary-value and deterministic property tests against Rust `checked_*` oracles.
6. Execute generated Core Wasm/Components in Wasmtime and record artifact/performance deltas.
7. Update RFC-0029, reports and M8 plan from measured evidence.

## 6. Changes

- Added semantic identities for `I64`, `U64` and `NumericError`, explicit literal construction, intrinsic type/arity checks, unchecked-`+` rejection and implicit-`Int`-narrowing rejection.
- Added IR fixed-width types, constants, checked operations and independent verifier mutation tests.
- Added dynamic parameter/local/result layouts, signed/unsigned overflow detection and comparison codegen.
- Added Component numeric result types and Canonical ABI indirect result returns.
- Added deterministic snapshots, 2,048 × 8 Node runtime oracle checks and 11 exact Wasmtime 46.0.1 Component cases.

## 7. Validation

Planned minimum:

```powershell
cargo test --offline --locked -p sico-semantics -p sico-ir -p sico-codegen-wasm
cargo fmt --all -- --check
cargo clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
.\tools\validate-step-0077.ps1
```

The focused Rust suite passed 37 tests. Node v25.1.0 executed 2,048 generated pairs across eight arithmetic/comparison paths plus six explicit boundaries. Wasmtime 46.0.1 executed 11 Component cases and returned `ok`, `err(overflow)`, `err(underflow)` and comparison values as expected. Full workspace quality gates are recorded in the completion commit.

## 8. Metrics

- fixed-width Core Wasm snapshot: 456 bytes;
- fixed-width Component snapshot: 1,209 bytes;
- 10 complete runtime-validator runs: median 354.206 ms, nearest-rank P95/max 460.088 ms, minimum 333.006 ms;
- runtime work per run: 16,384 generated oracle observations, six explicit Core boundaries and 11 separate Wasmtime Component processes.

These are local regression measurements, not startup SLAs. Detailed evidence is in [`fixed-width-dynamic-scalars-v0.md`](../reports/fixed-width-dynamic-scalars-v0.md).

## 9. Risks and follow-ups

- Runtime conversion from arbitrary `Int` values remains deliberately unimplemented; this step only accepts explicit in-range literal construction.
- The private Component result area is valid for the current synchronous, pointer-free numeric subset; general aggregate allocation and reentrancy belong to STEP-0079.
- STEP-0078 now owns general calls and control flow; STEP-0079 owns general aggregate Canonical ABI.

## 10. Audit links

- [`M8 plan`](../plans/M8-script-profile.md)
- [`STEP-0076`](./STEP-0076-script-profile-vertical-prototype.md)
- [`composition evidence`](../reports/script-profile-composition-v0.md)
- [`fixed-width evidence`](../reports/fixed-width-dynamic-scalars-v0.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`STEP-0078`](./STEP-0078-general-executable-control-codegen.md)
