# STEP-0220: M22 S6 — let-bound operation/call RHS

> - status: complete
> - phase: M22 S6 general-statement lowering after STEP-0219
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, Rust oracle regression, selfhost runner test, M22 status documents, this record

## 1. Objective

Support the first non-trivial `let` right-hand sides in the Sico-written
frontend: fixed-width bit operations, checked arithmetic operations and
typed user calls bound to a `let` name in straight-line functions.

## 2. Contract and mechanism

- `let x = I64.bit_and(a, b)`, `let x = I64.checked_add(a, b)` and
  `let x = callee(args)` lower the full expression tree inline and bind
  the name to the last emitted SSA value, exactly matching Rust
  `lower_straight_line`.
- The frontend classifies each RHS (`fixed_let_rhs_kind`) into
  `alias`/`literal`/`op`/`call`; op RHS reuses the existing operand
  check (typed `CALL-ARGUMENT`/`CALL-TYPE`/`CALL-ARITY`/range
  refusals), call RHS reuses the argument validator
  (`CALL-TARGET`/`CALL-ARITY`/`CALL-TYPE`). Anything else — including
  `I64.equal`/`less_than`, still outside the declared subset — stays a
  typed `ERR:E-SH-IR-STATEMENT` refusal.
- `fixed_let_count_between`, `fixed_let_binding_id` and
  `fixed_let_binding_kind` now account for operation/call RHS lets, so
  later operands, call arguments and return expressions resolve the
  aliased SSA ID and result type (scalar or Result JSON) without holes.
- `argument_values` was split into a thin wrapper (return-position
  behavior unchanged) and `argument_values_base(…, value_base)` with an
  explicit constant base. Nested calls inside `argument_instructions`
  now take their JSON from the running base instead of recomputing it
  from the let count; this is byte-identical on the existing corpus and
  keeps nested-call argument IDs correct inside let-bound calls.
- Instruction ranges follow Rust `token_range`: an op/call instruction
  spans its first source token through the closing parenthesis.

## 3. Executable evidence

Six new real-runner differentials are deserialized, independently
verified, canonicalized and compared byte-for-byte with Rust
`lower_core`: parameter-operand bit-op let, checked-op let with Result
type, user-call let with fixed-width literal arguments, chained
literal-operand op let feeding an alias and a return op, call let with
parameter + let-bound arguments, and a Result-returning callee let.
Four new typed refusals are pinned: unresolved op operand, undeclared
let-call target, checked Result bound into an `I64` return, and the
`I64.equal` let RHS as a declared-subset refusal (Rust accepts; the
refusal is declared and disjoint).

The cumulative positive differential set is now fifty-two programs.
The selfhost parser suite stays 2/2 green on the real Windows x64
runner; the Rust oracle suite is 14/14.

Reproducible validators:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline -p sico-ir --test core_lowering
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser
& "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --locked --offline -p sico-ir --all-targets -- -D warnings
& "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --all-targets -- -D warnings
```

Evidence class: `internal-fixture`, Windows x64 GNU.

## 4. Residuals

`mutable set`, general `if`/loops and non-frozen match shapes remain
typed refusals; expression trees wider than the declared subset
(parens grouping, sibling-constant nesting beyond tested shapes,
`equal`/`less_than`) stay refused or are pinned by later steps. This
step is not codegen, bootstrap closure or any UI support claim.
