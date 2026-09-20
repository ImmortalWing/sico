# STEP-0238: M22 nested call arguments in cell mode

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0237
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0238.ps1`

## 1. Objective

Support nested calls inside cell-mode call arguments (`set x = f(g(y), x)`),
byte-identical to the Rust `lower_core` general-CFG path.

## 2. Contract and mechanism

A cell-mode call argument segment that is itself a call resolves through the
frozen argument machinery: `argument_values` validates and resolves the nested
callee's parameter/constant/fixed-literal arguments (arity and position type
refusals included), `argument_instruction_count` sizes the segment, and
`argument_instructions` emits the nested instructions and the nested call with
the segment's source span. The nested value is the emission base plus the
segment count minus one; the base advances by the segment count. Call-shaped
segments are exempt from the single-atom width check, and a `sccall_seg_advance`
counter now drives emission-base progression uniformly. Cell reads inside
nested calls remain outside the executed subset (`argument_values` fails
closed on them).

## 3. Executable evidence

Four positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and match
Rust `lower_core` byte-for-byte: a set whose call mixes a nested param call
with a cell argument, two nested calls in one argument list, and a let
right-hand side with a nested call feeding a later set. Two negative fixtures
prove an unknown nested callee and a wrong-type nested result fail closed with
typed refusals. The cumulative positive set is 100 programs.

## 4. Residuals

Cell reads inside nested call arguments and operation/call return expressions
in cell mode remain outside the executed subset. Nested control flow,
alternate match-arm shapes, full-corpus lowering, S5 codegen and S6 bootstrap
remain open.
