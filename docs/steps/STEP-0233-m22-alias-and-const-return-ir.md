# STEP-0233: M22 alias bindings and bare constant returns in the straight environment

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0232
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0233.ps1`

## 1. Objective

Unify alias bindings (`let y = param`) and bare scalar constant returns
(`return 7` / `return true` / `return "text"`) with the loop-driven straight
binding environment, closing the remaining gaps between the environment and
the Rust `lower_core` straight-line subset.

## 2. Contract and mechanism

A single-atom binding whose atom resolves to a parameter or a prior local is
an alias: it emits no instruction, records the aliased value id and kind, and
does not consume an SSA id — the next statement keeps the same next-id.
Instruction emission now tracks an emitted-instruction counter, so commas
appear only between real instructions and an all-alias body lowers to an
empty instruction list.

A return consisting of a single canonical magnitude, `true`/`false` or a
string literal emits one scalar const instruction (same JSON shape and token
span as STEP-0232 bindings) after the body's instructions; the terminator
references its id and the kind is checked against the declared return kind.
Return resolution order is parameter, then local, then constant, so names
shadow constants exactly as in Rust `lower_core`.

Unknown bare atoms, invalid escapes, unclosed strings, non-canonical
magnitudes and return-kind mismatches remain typed refusals.

## 3. Executable evidence

Five positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and match
Rust `lower_core` byte-for-byte: a two-hop parameter alias chain feeding a
fixed-width operation; a parameter alias returned after an operation on it; a
local-to-local alias beside a constant with a bare integer return; a bare
bool constant return; and a bare unicode string constant return. The
cumulative positive set is 69 programs.

## 4. Residuals

Fixed-width literal and typed user call returns from long blocks are not yet
unified. Mutation, nested control flow, alternate match-arm shapes,
full-corpus lowering, S5 codegen and S6 bootstrap remain open.
