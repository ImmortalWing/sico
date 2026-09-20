# STEP-0234: M22 fixed-width literal and typed user call returns in the straight environment

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0233
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0234.ps1`

## 1. Objective

Unify fixed-width literal returns (`return I64.literal(3)`) and typed user
call returns (`return f(x, 1)`) with the loop-driven straight binding
environment, closing the last return-shape gaps of the S4 environment.

## 2. Contract and mechanism

A multi-token return is now classified before the scalar-constant path: a
call shape `name(...)` resolves through the environment argument machinery
(`environment_call_arguments` with local bindings, `argument_instructions`
for source-ordered const/nested-call emission, position type checking);
literal and nested-call instructions emit first and the call result takes
the next SSA id, with the whole expression span as its range. A fixed
literal shape `I64/U64.literal(N)` emits one `const_i64`/`const_u64`
through the new `fixed_literal_const_instruction` helper: positive
magnitudes range over the digits only, negative magnitudes over the sign
through the digits, with the same decimal bounds and `E-SH-IR-I64-RANGE` /
`E-SH-IR-U64-RANGE` refusals as the frozen return paths.

`environment_call_arguments` additionally accepts nested call segments:
the nested callee's declared return kind is position-checked, its value is
the emission base plus its instruction count minus one, and the base
advances by that count — mirroring `argument_values` while keeping local
resolution. Segment width validation is skipped for call-shaped segments.

Unknown callees, arity and type mismatches, malformed shapes, overflow and
unresolved names remain typed refusals.

## 3. Executable evidence

Seven positive fixtures run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte: fixed literal returns (I64 positive,
U64 max) after bindings; a typed call return consuming a local and a fixed
literal; a param-only call return beside an unused constant; a call return
with a nested call and a local argument; and a bool/text call return with
a local. Four negative fixtures prove unknown return callees, overflowing
fixed literals, wrong-type call arguments and call arity mismatches fail
closed with typed refusals. The cumulative positive set is 76 programs.

## 4. Residuals

Fixed-width operation returns from long blocks remain outside the executed
subset. Mutation, nested control flow, alternate match-arm shapes,
full-corpus lowering, S5 codegen and S6 bootstrap remain open.
