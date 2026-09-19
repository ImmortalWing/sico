# STEP-0217: M22 S6 — let-bound fixed and checked operands

> - status: complete
> - phase: M22 S6 general-statement lowering after STEP-0216
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, Rust oracle regression, selfhost runner test, M22 status documents, this record

## 1. Objective

Make straight-line fixed-width `let` values usable by later fixed-width
and checked arithmetic expressions while preserving Rust's source-order
SSA IDs when a return expression also contains inline literals.

## 2. Contract and mechanism

- Fixed-operation operand resolution now searches parameters, then prior
  fixed-literal `let` bindings, then inline fixed literals. A binding must
  precede the return expression and match the receiver width.
- Let-bound values keep their existing IDs. Inline return-expression
  literals begin after all parameters and preceding let instructions;
  the final operation follows both. This preserves left-to-right Rust
  evaluation order without renumbering earlier values.
- `bit_and`/`bit_or`/`bit_xor`/`shl`/`shr` and
  `checked_add`/`checked_sub`/`checked_mul`/`checked_div` share the same
  resolver. Existing no-let programs retain their frozen bytes.
- Width mismatch remains `ERR:E-SH-IR-CALL-TYPE`; unresolved and wider
  let RHS shapes remain typed refusals and produce no IR.

## 3. Executable evidence

Two new real-runner differentials are deserialized, independently
verified, canonicalized and compared byte-for-byte with Rust
`lower_core`:

1. two `I64` let bindings feed `I64.bit_and`;
2. one `U64` let binding plus an inline `U64.literal(2)` feed
   `U64.checked_div`, pinning IDs across both instruction regions.

The cumulative positive differential set is now forty-two programs. A
`U64` let passed to an `I64` operation matches Rust's refusal class via
typed `ERR:E-SH-IR-CALL-TYPE`. The selfhost parser suite remains 2/2
green on the real Windows x64 runner.

Reproducible validators:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline -p sico-ir --test core_lowering
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser
```

Evidence class: `internal-fixture`, Windows x64 GNU.

## 4. Residuals

Calls and other expressions on a let RHS, mutable `set`, general
`if`/loops/match blocks and full-corpus lowering remain open. STEP-0218
subsequently allows fixed-width let values as user-call arguments. This
step is not codegen or bootstrap closure and makes no UI support claim.
