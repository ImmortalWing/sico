# STEP-0215: M22 S6 — checked-Result all-return match IR

> - status: complete
> - phase: M22 S6 typed-IR expansion after STEP-0214
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `selfhost/parser.sico`, selfhost runner test, M22 status documents, this record

## 1. Objective

Make the Sico-written frontend mirror the verifier-proven Rust lowering
for the first bounded control-flow slice: an all-return match over a
computed `I64` or `U64` checked arithmetic `Result` with `ok(binding)`
and `error(binding)` payload arms.

## 2. Contract and mechanism

- The accepted slice is deliberately exact: one checked operation over
  two same-width parameters, followed by `ok(binding)` returning that
  binding and `error(binding)` returning a same-width non-negative fixed
  literal. Both arms bind payloads. Extra statements, missing arms,
  wildcard payloads, other subjects or other return shapes remain a
  typed `ERR:E-SH-IR-CONTROL` refusal.
- The guest emits the STEP-0137/Rust-oracle structure pinned by
  STEP-0214: checked operation and typed `write_local` in block 0;
  `match` terminator with canonical variant patterns; arm-local
  `read_local` plus `project`; source-ordered value IDs and exact source
  ranges.
- Receiver/operand/return types, parameter resolution, literal bounds,
  punctuation and every token in the frozen shape are checked before
  JSON emission. No fallback IR is emitted for a near match.
- The existing direct-return lowering path and its byte output remain
  unchanged.

## 3. Executable evidence

Two new real-runner positives are deserialized, independently verified,
canonicalized and compared byte-for-byte with Rust `lower_core`:

1. `I64.checked_add` with an `I64.literal(0)` error arm;
2. `U64.checked_div` with a `U64.literal(1)` error arm.

The cumulative positive differential set is now thirty-nine programs.
One additional fixture inserts a `let` into an arm and proves the wider
shape remains the typed `ERR:E-SH-IR-CONTROL` refusal. The complete
selfhost parser suite remains 2/2 green on the real Windows x64 runner.

Reproducible validator:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline --manifest-path .\runner\sico-runner\Cargo.toml --test selfhost_parser
```

Evidence class: `internal-fixture`, Windows x64 GNU.

## 4. Residuals

At this step boundary general statement blocks, match subjects beyond
this checked binary shape, wildcard/no-payload arms, arbitrary arm
expressions, nested calls as operation operands and full-corpus lowering
remained open. STEP-0216 subsequently opens the first straight-line
fixed-width `let` SSA slice. This step is not codegen or bootstrap
closure and makes no UI support claim.
