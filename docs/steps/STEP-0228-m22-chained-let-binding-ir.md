# STEP-0228: M22 chained let-binding IR

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0227
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0228.ps1`

## 1. Objective

Prove that a straight-line block can carry a typed SSA binding from one
`let` expression into the next, rather than only lowering an isolated
binding before return.

## 2. Contract and mechanism

The accepted shape is exactly two fixed-width bitwise or shift bindings
followed by return of the second name. The first expression uses the
existing parameter/literal operand path. The second expression may
consume the first binding and same-width parameters. Its result id
follows all first-expression constants and the first operation. No local
cell is emitted. Source ranges are anchored independently per physical
statement so newline sentinel tokens cannot leak into byte offsets.

Unknown bindings, width mismatches, duplicate binding names, more than
two lets and other statement shapes remain typed refusals.

## 3. Executable evidence

One fixture chains I64 `bit_and` into `bit_or`; another chains U64 `shl`
into `bit_xor` with the prior result in the right operand. Both run
through the Sico-built parser Component, deserialize, pass the independent
verifier, canonically round-trip and match Rust `lower_core` byte-for-byte.
Negative fixtures prove that an I64 binding cannot feed a U64 operation
and that a third operand is rejected instead of ignored. The cumulative
positive set is 50 programs.

## 4. Residuals

This is still a bounded straight-line block. Arbitrary binding counts,
second-expression literals or calls, mutation, nested control flow,
alternate match-arm shapes, full-corpus lowering, S5 codegen and S6
bootstrap remain open.
