# STEP-0219: M22 S6 — straight-line `let` alias RHS

> - status: complete
> - phase: M22 S6 general-statement lowering after STEP-0218
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, Rust oracle regression, selfhost runner test, M22 status documents, this record

## 1. Objective

Support the first non-literal `let` RHS in the Sico-written frontend:
an alias of a function parameter or a previously declared supported let
binding, with no artificial SSA instruction.

## 2. Contract and mechanism

- `let alias = parameter` and `let alias = prior_let` resolve to the
  existing value ID and type. Alias chains resolve backward only; unknown
  or forward names remain `ERR:E-SH-IR-STATEMENT`.
- Alias lets emit zero instructions, exactly matching Rust straight-line
  lowering. Fixed-literal lets continue to emit one constant instruction.
- The frontend now tracks source let statement count separately from
  emitted let instruction count. Operation/call constant bases use the
  latter, preventing holes when aliases appear between emitting lets and
  later expressions.
- Duplicate binding names remain refused. Existing fixed/checked and
  user-call consumers automatically receive the resolved aliased ID.

## 3. Executable evidence

Two new real-runner differentials are deserialized, independently
verified, canonicalized and compared byte-for-byte with Rust
`lower_core`:

1. a parameter alias returns the original parameter ID with an empty
   instruction list;
2. a literal binding followed by an alias feeds `I64.bit_or` with an
   inline literal, proving the alias does not reserve an ID.

The second fixture initially triggered verifier `NonCanonicalId` and
`UndefinedValue` failures because the operation base used let statement
count (2) instead of emitted instruction count (1). The implementation
now separates those counts and the retained fixture pins the correction.

The cumulative positive differential set is now forty-six programs. An
unresolved alias remains a typed refusal. The selfhost parser suite stays
2/2 green on the real Windows x64 runner.

Reproducible validators:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline -p sico-ir --test core_lowering
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser
```

Evidence class: `internal-fixture`, Windows x64 GNU.

## 4. Residuals

Calls and operations that themselves form a let RHS, non-fixed values,
mutable `set`, general `if`/loops/match blocks and full-corpus lowering
remain open. This step is not codegen or bootstrap closure and makes no
UI support claim.
