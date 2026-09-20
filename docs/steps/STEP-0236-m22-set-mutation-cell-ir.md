# STEP-0236: M22 set/mutation cell-mode lowering in the Sico frontend

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0235
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0236.ps1`

## 1. Objective

Support `set` mutation in the Sico-written lowering parser by reproducing
the Rust `lower_core` general-CFG cell mode: any function containing `set`
lowers every `let` to a mutable local cell, every read to `read_local`,
and every assignment to `write_local`, byte-identically.

## 2. Contract and mechanism

`set` forces Rust `lower_core` onto the general CFG path where `let`
emits the right-hand-side instructions followed by `write_local` (Unit,
whole-line range) into a fresh cell, reads of cell-bound names emit
`read_local` with the cell type and the read token's span, and `set`
emits its right-hand side plus `write_local` into the existing cell. The
function JSON gains a `locals` table (name, kind, binding-name span) in
creation order; the block and function ranges stay the frozen spans.

The new `straight_cell_function_ir` mirrors this for a bounded statement
subset: right-hand sides and returns may be parameters, prior cells,
scalar constants, or fixed-width literals. `set` requires an existing
cell (`E-SH-IR-SET-CELL`) of matching kind (`E-SH-IR-TYPE-MISMATCH`);
redeclaring a cell is `E-SH-IR-CELL-REDEFINED`. Multi-token operation and
call right-hand sides in cell mode remain outside the executed subset.

## 3. Executable evidence

Seven positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte: self-assign read/write, int and
bool constant cells, fixed-literal cells, cell-to-cell assignment, and a
text cell reassigned to a string constant. Three negative fixtures prove
set-without-let, set kind mismatch and cell redeclaration fail closed with
typed refusals (Rust refuses the same sources). The cumulative positive
set is 88 programs.

## 4. Residuals

Operation and call right-hand sides in cell mode (Rust accepts them via
the general path) remain outside the executed subset. Nested control
flow, alternate match-arm shapes, full-corpus lowering, S5 codegen and
S6 bootstrap remain open.
