# STEP-0216: M22 S6 — straight-line fixed-width `let` SSA

> - status: complete
> - phase: M22 S6 general-statement lowering after STEP-0215
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, Rust oracle regression, selfhost runner test, M22 status documents, this record

## 1. Objective

Open the general-statement path in the Sico-written frontend by lowering
straight-line fixed-width `let` bindings as SSA values rather than
rejecting every function containing `let`.

## 2. Contract and mechanism

- `=` is now an explicit selfhost lexer punctuation token. This was a
  real prerequisite gap hidden while lowering only inspected return
  expressions; the existing parser-summary fixture proves adding the
  token does not drift its frozen output.
- The accepted slice is an arbitrary ordered sequence of positive
  `I64.literal(N)` or `U64.literal(N)` bindings followed by a direct
  return of a parameter or one of those bindings. Each literal receives
  its source-ordered SSA value ID after the function parameters.
- Literal canonicality and fixed-width bounds are checked before JSON
  emission. Duplicate names and every other `let` expression stay the
  typed `ERR:E-SH-IR-STATEMENT` refusal; no fallback IR is emitted.
- Instruction ranges cover the magnitude, matching Rust's fixed-literal
  lowering. Unused bindings remain emitted in source order, also matching
  Rust rather than applying an undeclared optimization.

## 3. Executable evidence

The Rust regression `straight_line_fixed_let_bindings_lower_to_ssa_values`
pins two source-ordered constants (`ConstI64(7)`, `ConstU64(9)`) and a
return of value ID 1. The same mixed-width source is executed through the
Sico-written parser component, deserialized, independently verified,
canonicalized and compared byte-for-byte with Rust `lower_core`.

The cumulative positive differential set is now forty programs. A
`let copy = value` fixture proves non-literal bindings remain a typed
refusal. The complete selfhost parser suite remains 2/2 green on the real
Windows x64 runner.

Reproducible validators:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline -p sico-ir --test core_lowering
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser
```

Evidence class: `internal-fixture`, Windows x64 GNU.

## 4. Residuals

Binding/call/fixed-operation expressions on the right-hand side of a
`let`, mutable `set`, general `if`/loops/match blocks and full-corpus
lowering remain open. STEP-0217 subsequently allows these fixed-literal
let values as fixed/checked return-expression operands. This step is not
codegen or bootstrap closure and makes no UI support claim.
