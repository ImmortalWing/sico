# STEP-0193: M22 S6 — Sico lowering: parser extracts function names

> - status: complete — **S6 opened: the parser now extracts function names
> from the token stream (`fn:main;`), the first lowering-level shape**
> - phase: M22 S6 (the lower slice of the L2 compiler self-host)
> - completed: 2026-09-14
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, this STEP record

## 1. What was done

`selfhost/parser.sico` grows from STEP-0192's keyword counting into the
first **lowering-level** shape: walking the token stream, when it sees
the `function` keyword it reads the next token as the function name and
emits a `fn:<name>;` summary. On the probe source `function main() returns
Int: …` the parser emits `fn:main;`.

This is the seam where parsing becomes lowering: the token stream is now
structured into a declaration summary (function identity), the input to
the eventual IR construction. The full lower (expression trees, blocks,
the typed IR the Rust verifier checks) and codegen (wasm emission) are
the remaining S6 slices — this step proves the parser→lowering handoff
in Sico.

## 2. Validation

- `selfhost_parser.rs` 1/1: `fn:main;` byte-exact.

## 3. Residuals (the S6 completion boundary)

- Full **lowering to typed IR**: expressions, blocks, control flow — the
  parser must build the IR-shaped tree, verified against the Rust IR
  verifier (the M22 plan's oracle).
- **Codegen**: the wasm emission from the IR — the largest remaining
  slice, gated on the IR being verifier-clean.
- The bootstrap closure (S6's exit) needs the Sico compiler to compile
  its own source; that is multiple slices beyond this handoff proof.
