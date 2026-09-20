# STEP-0218: M22 S6 — let-bound user-call arguments

> - status: complete
> - phase: M22 S6 general-statement lowering after STEP-0217
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, Rust oracle regression, selfhost runner test, M22 status documents, this record

## 1. Objective

Allow prior fixed-width `let` SSA values to flow into direct user-function
arguments while preserving source-order instruction IDs, argument order,
inline-constant placement and exact source ranges.

## 2. Contract and mechanism

- User-call argument resolution now searches parameters, prior fixed
  `let` bindings, constants/fixed literals and nested calls in that order.
  Let arguments emit no duplicate instruction and carry their original
  SSA ID.
- The call-argument instruction base begins after all preceding let
  instructions. Inline argument constants and nested calls retain their
  existing left-to-right recursive ordering; the outer call follows them.
- The straight-line dispatcher now admits a call return after supported
  lets and prepends the let instructions to the canonical block.
- Argument arity and type checks are unchanged. A let-bound value of the
  wrong width remains `ERR:E-SH-IR-CALL-TYPE` and emits no IR.

## 3. Executable evidence

Two new real-runner differentials are deserialized, independently
verified, canonicalized and compared byte-for-byte with Rust
`lower_core`:

1. two let bindings passed in reversed argument order, pinning call
   arguments `[1,0]` without re-emission;
2. one let binding followed by an inline fixed literal, pinning the
   literal and call IDs after the let region.

The first differential exposed and fixed a real range bug: searching a
numeric magnitude from the beginning of a let line could match the `6`
inside `I64` before the actual `literal(6)`. Magnitude lookup is now
anchored after the `.literal` token; the retained fixture prevents
regression.

The cumulative positive differential set is now forty-four programs. A
wrong-width let argument matches Rust's refusal class through typed
`ERR:E-SH-IR-CALL-TYPE`. The selfhost parser suite remains 2/2 green on
the real Windows x64 runner.

Reproducible validators:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline -p sico-ir --test core_lowering
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser
```

Evidence class: `internal-fixture`, Windows x64 GNU.

## 4. Residuals

Calls and operations on a let RHS, non-fixed values, mutable `set`,
general `if`/loops/match blocks and full-corpus lowering remain open.
STEP-0219 subsequently supports parameter/prior-let aliases as the first
non-literal RHS. This step is not codegen or bootstrap closure and makes
no UI support claim.
