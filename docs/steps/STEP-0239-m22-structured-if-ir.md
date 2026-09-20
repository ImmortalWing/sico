# STEP-0239: M22 structured `if` lowering in the Sico frontend

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0238
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0239.ps1`

## 1. Objective

Lower structured `if`/`else` through the Sico-written frontend, emitting the
Rust `lower_core` general-CFG block shapes byte-identically: branch
terminators, arm/join blocks with `if`-header ranges, cell-mode statements,
and the unreachable join block for return-arm forms.

## 2. Contract and mechanism

`control_if_function_ir` handles one top-level `if` per function. The
condition is an `I64/U64.equal/less_than` fixed predicate whose operands are
parameters, prior cells (`read_local` in the entry block) or same-width
fixed literals; it lowers to `equal_fixed`/`less_fixed` (Bool) with the
condition span. Two arm forms are supported, each in both set-form and
return-arm variants:

- set form: arms contain `set` statements (`write_local`, arm line spans),
  both arms jump to a join block that runs trailing sets and the final
  return; without `else`, an empty else block jumps to the join.
- return form: each arm is a single `return` of a parameter, cell, scalar
  constant or fixed literal (kind-checked against the declared return kind);
  the join block is emitted as unreachable.

Value ids are globally sequential in emission order (entry, then, else,
join); arm/join/unreachable blocks carry the `if` header line range; the
entry block keeps the function range; the `locals` table is omitted when
empty (canonical JSON drops it via `skip_serializing_if`). Nested `if`,
mixed arm forms, and operation/call right-hand sides inside arms remain
typed refusals or outside the executed subset.

## 3. Executable evidence

Seven positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte: if/else with parameter-return arms
(branch plus unreachable join), cell-conditioned set arms with and without
else, a cell operand in the condition, fixed-literal return arms, and
multi-statement arms. Two negative fixtures prove Int constants returned
from an I64 function (Rust `IMPLICIT_NUMERIC_CONVERSION`) and wrong-width
condition operands fail closed with typed refusals. The cumulative positive
set is 107 programs.

## 4. Residuals

`while` loops, nested `if`, operation/call right-hand sides inside arms, and
if-after-if sequences remain outside the executed subset. Full-corpus
lowering, S5 codegen and S6 bootstrap remain open.
