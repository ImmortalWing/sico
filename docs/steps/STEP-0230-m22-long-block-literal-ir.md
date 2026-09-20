# STEP-0230: M22 fixed-width literals in long straight blocks

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0229
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0230.ps1`

## 1. Objective

Unify fixed-width literal operands with the loop-driven straight binding
environment so constants are not restricted to the one/two-binding
special paths.

## 2. Contract and mechanism

Every operation in a three-or-more-binding straight block may now consume
same-width parameters, prior bindings or I64/U64 fixed literals in either
operand position. Operand widths drive exact binary-shape validation.
Literal const instructions emit in source order before their operation;
the binding records the operation result after those constants, and the
next statement starts from the following SSA id.

Range overflow, wrong-width literals, unknown operands, extra operands
and malformed literal shapes remain typed refusals.

## 3. Executable evidence

An I64 fixture mixes a negative literal, a left literal and a later right
literal across three dependent bindings. A U64 fixture starts with two
literals, then consumes the result with another literal and a parameter.
Both run through the Sico-built parser Component, deserialize, pass the
independent verifier, canonically round-trip and match Rust `lower_core`
byte-for-byte. A negative fixture proves U64 overflow inside a later
binding fails closed. The cumulative positive set is 54 programs.

## 4. Residuals

Typed user calls and scalar constant bindings are not yet unified into
the long-block environment. Mutation, nested control flow, alternate
match-arm shapes, full-corpus lowering, S5 codegen and S6 bootstrap
remain open.
