# STEP-0077: Fixed-width dynamic scalar representation

> - status: planned
> - phase: M8
> - started: -
> - completed: -
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

The frozen direction is distinct fixed-width types, not backend-only narrowing of `Int`. Before implementation, compare explicit per-operation IR variants against one typed generic numeric operation; choose the smaller verifier/codegen surface that still preserves signedness and stable overflow diagnostics.

## 5. Plan

1. Audit semantic builtin/type handling, integer-literal typing and existing no-implicit-conversion rules.
2. Freeze IR JSON names and checked arithmetic/comparison operation set.
3. Add verifier mutations for signedness, operand/result mismatch and illegal `Int` narrowing.
4. Implement dynamic Wasm locals/parameters/results and checked overflow/underflow paths.
5. Add boundary-value and deterministic property tests against Rust `checked_*` oracles.
6. Execute generated Core Wasm/Components in Wasmtime and record artifact/performance deltas.
7. Update RFC-0029, reports and M8 plan from measured evidence.

## 6. Changes

None yet. This record reserves and bounds the next M8 implementation step after STEP-0076.

## 7. Validation

Planned minimum:

```powershell
cargo test --offline --locked -p sico-semantics -p sico-ir -p sico-codegen-wasm
cargo fmt --all -- --check
cargo clippy --offline --locked --workspace --all-targets --all-features -- -D warnings
```

Runtime fixtures must execute with exact Wasmtime 46.0.1 evidence; compile-only tests are insufficient for completion.

## 8. Metrics

No STEP-0077 implementation metrics yet.

## 9. Risks and follow-ups

- Integer literals currently infer as `Int`; explicit construction/conversion syntax may expose a language-level decision that requires RFC-0029 revision rather than an implementation shortcut.
- Checked arithmetic must not become trap-only behavior that loses typed failure information.
- STEP-0078 depends on dynamic value storage from this step but owns general calls and control flow.

## 10. Audit links

- [`M8 plan`](../plans/M8-script-profile.md)
- [`STEP-0076`](./STEP-0076-script-profile-vertical-prototype.md)
- [`composition evidence`](../reports/script-profile-composition-v0.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
