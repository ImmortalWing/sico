# STEP-0199: M22 Sico-native deterministic Core Wasm seam

> - status: complete (single constant-return codegen seam; not general codegen or bootstrap)
> - phase: M22 codegen, first Sico-emitted executable bytes
> - completed: 2026-09-16
> - owners: autonomous-agent
> - artifacts: `selfhost/compiler.sico`, `runner/sico-runner/tests/selfhost_compiler.rs`, `tools/validate-step-0199.ps1`, this STEP record

## 1. What was done

The Sico-written compiler gained a bounded `--emit-core-hex` transport for the
accepted constant-return fixture. It emits the complete deterministic Core Wasm
byte vector as canonical lowercase hex; the native test decodes that data-only
transport, compares every byte with RFC-0011's Rust backend output and validates
the result with `wasmparser`.

The emitted function body includes the Rust backend's real structured-CFG
dispatcher/local layout, not a hand-written semantically equivalent shortcut.
The first attempted minimal body was correctly rejected by byte comparison even
though it was valid Wasm; the recorded output was corrected to the canonical
backend bytes.

## 2. Evidence

- `selfhost_compiler` 3/3 green through the real runner.
- canonical IR: six accepted scalar shapes + one same-length refusal.
- Core Wasm: Sico-emitted decoded bytes equal Rust output byte-for-byte and pass
  independent `wasmparser` validation.

Evidence class: `internal-fixture`, Windows x64 GNU runner.

## 3. Honest boundary

This STEP proves the guest→binary→validator seam only. The initial codegen body
is fixed to the frozen `answer() -> 42` shape; it does not yet derive arbitrary
sections/instructions from typed IR, emit a Component wrapper, compile the
compiler source, or satisfy A=B=C. Those remain binding M22 gates.
