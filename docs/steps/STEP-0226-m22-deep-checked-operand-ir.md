# STEP-0226: M22 recursive checked-arithmetic operand IR

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0225
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0226.ps1`

## 1. Objective

Prove that checked-arithmetic operands use the recursive expression path,
not a one-level special case.

## 2. Contract and mechanism

The STEP-0225 operand splitter and validator recurse through user-call
arguments. Instruction counts are accumulated inside-out, nested
constants and calls emit in source order, and the outer checked
instruction consumes the final nested result id. Parenthesis matching
continues to split only at the outer checked call's comma.

## 3. Executable evidence

One fixture lowers `id(id(a))`; another lowers
`id(U64.literal(3))`. Both run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte. The cumulative positive set is
46 programs.

## 4. Residuals

The recursive expression path is now demonstrated inside checked
operands. Alternate match-arm shapes, non-return bodies, general
statements, full-corpus lowering, S5 codegen and S6 bootstrap remain
open.
