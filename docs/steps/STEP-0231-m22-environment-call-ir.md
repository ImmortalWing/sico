# STEP-0231: M22 typed user calls in the loop-driven straight environment

> - status: complete
> - phase: M22 S4 typed-IR expansion after STEP-0230
> - completed: 2026-09-18
> - owners: autonomous-agent
> - evidence class: `internal-fixture`
> - artifacts: `selfhost/parser.sico`, `runner/sico-runner/tests/selfhost_parser.rs`, `tools/validate-step-0231.ps1`

## 1. Objective

Unify typed user call expressions with the loop-driven straight binding
environment so call bindings are not restricted to the one/two-binding
special paths.

## 2. Contract and mechanism

Every `let` binding in a three-or-more-binding straight block may now be
either a fixed-width operation (STEP-0230) or a typed user call. Call
arguments are comma-segmented with nested-parenthesis awareness; each
segment resolves to a same-width parameter, a prior local binding or an
I64/U64/int/bool/text constant, and is type-checked against the callee's
declared parameter kind in position order. Literal and nested-call
instructions emit in source order before the call instruction; the call
result takes the next SSA id after those constants, the binding records it,
and the next statement starts from the following SSA id.

Unknown call targets, arity mismatches, wrong-type arguments, extra tokens
after the closing parenthesis and malformed argument shapes remain typed
refusals, as do forward local references inside argument lists.

## 3. Executable evidence

An I64 fixture mixes an operation and a call consuming a prior binding plus
a fixed literal. A three-arity Text fixture chains calls whose arguments
alternate parameters, bool/int/text constants and prior call results. Two
single-parameter fixtures chain `identity` over a parameter and over a
U64 literal. All four run through the Sico-built parser Component,
deserialize, pass the independent verifier, canonically round-trip and
match Rust `lower_core` byte-for-byte. Negative fixtures prove unknown
targets, wrong-type long-block arguments and overflowing literals fail
closed with typed refusals. The cumulative positive set is 58 programs.

## 4. Residuals

Scalar constant (`Int`/`Bool`/`Text`) bindings are not yet unified into the
long-block environment. Mutation, nested control flow, alternate match-arm
shapes, full-corpus lowering, S5 codegen and S6 bootstrap remain open.
