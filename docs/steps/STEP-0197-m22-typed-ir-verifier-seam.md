# STEP-0197: M22 typed-IR verifier seam

> - status: complete (bounded scalar identity/constant slice; not bootstrap closure)
> - phase: M22 lowering, first canonical `sico.ir.v0` differential
> - completed: 2026-09-16
> - owners: autonomous-agent
> - artifacts: `selfhost/compiler.sico`, `runner/sico-runner/tests/selfhost_compiler.rs`, `tools/validate-step-0197.ps1`, this STEP record

## 1. What was done

The Sico-written compiler now crosses the actual typed-IR boundary instead of
printing an outline summary. For a bounded scalar slice it tokenizes and checks
the canonical source shape, derives function/parameter names, scalar types,
source length and parameter/literal spans, and emits canonical `sico.ir.v0`
JSON.

The native harness treats that JSON as untrusted data: it deserializes to
`sico_ir::Module`, runs the permanent Rust verifier, canonically reserializes it
and requires byte equality with the guest output. The accepted fixtures cover
`Int`, `I64`, `U64` and `Bool` identity functions, a constant-`Int` function,
and a two-parameter `AddInt` expression. The constant fixture also proves that the verified guest IR and the
Rust-lowered IR feed the deterministic Rust Component backend to identical
bytes.

An equal-length punctuation mutation is rejected as typed `invalid-input`; the
compiler does not accept a token-equivalent but grammatically different source.

## 2. Evidence

- Real runner builds and executes `selfhost/compiler.sico` as a Script-profile
  Component.
- `selfhost_compiler` differential tests green; six accepted scalar cases each match
  Rust canonical IR byte-for-byte, and the refusal case produces no IR.
- Rust verifier accepts every guest module and its canonical reserialization is
  byte-identical.
- Deterministic Component bytes match for the constant-return codegen handoff.

Evidence class: `internal-fixture`, Windows x64 GNU runner. This STEP does not
claim Sico-native codegen, multi-function/control-flow coverage or bootstrap.

## 3. Residuals

- Expand AST/lowering coverage to calls, blocks, control flow and the constructs
  used by the compiler source itself.
- Implement deterministic Component emission in Sico. Current byte equality at
  codegen is a handoff check through the Rust backend, not Sico-native codegen.
- Close A=B=C self-compilation, corpus replay, M7 package verification and the
  ADR-0015 measurement table.
