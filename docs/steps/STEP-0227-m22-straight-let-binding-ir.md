# STEP-0227: M22 straight-line let-binding IR

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0226
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0227.ps1`

## 1. Objective

Open the first bounded general-statement slice: lower one immutable `let`
binding followed by `return` of that binding, without inventing a local
cell or changing Rust `lower_core` semantics.

## 2. Contract and mechanism

The accepted shape is exactly one fixed-width bitwise or shift expression
bound by `let`, followed by a single return of the bound name. The
expression uses the existing I64/U64 operand validator, literal emitter
and SSA allocator. The binding is therefore a pure SSA alias: literal
instructions, when present, precede the operation and the terminator
returns the operation result directly. Extra statements, duplicate lets,
unsupported expressions and unresolved return names remain typed
refusals.

## 3. Executable evidence

An I64 `bit_and` fixture covers two parameter operands. A U64 `shl`
fixture covers a fixed-width literal operand and its preceding const
instruction. Both run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte. A negative fixture proves that an
unresolved returned binding fails closed. The cumulative positive set is
48 programs.

## 4. Residuals

This is not general block lowering. Multiple lets, arbitrary let
expressions, mutation, nested control flow, alternate match-arm shapes,
the full compiler corpus, S5 codegen and S6 bootstrap remain open.
