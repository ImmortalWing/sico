# STEP-0214: M22 S6 — checked-Result match oracle investigation

> - status: complete
> - phase: M22 S6 lowering investigation after STEP-0213
> - completed: 2026-09-18
> - owners: autonomous-agent
> - artifacts: `crates/sico-ir/tests/core_lowering.rs`, M22 status documents, this record

## 1. Objective

Resolve STEP-0213's claim that Rust lowering of a computed checked
arithmetic `Result` match was rejected by the independent verifier before
the Sico-written frontend mirrors that shape.

## 2. Finding

The claimed Rust-oracle mismatch does not reproduce. A minimal source
function matching `I64.checked_add(left, right)` with `ok(value)` and
`error(reason)` payload arms lowers successfully and passes the independent
verifier. The canonical Rust lowering uses the existing STEP-0137 shape:

1. emit the checked operation in block 0;
2. spill its computed `Result[I64, NumericError]` into a typed local;
3. terminate block 0 with the two-arm match;
4. read the local separately inside each arm before projecting `ok` or
   `error`.

The earlier ad-hoc probe was therefore not valid evidence about the Rust
oracle. Its exact transient IR was not retained, so this step does not
invent a more specific cause. The permanent regression test pins the
actual accepted source and the required spill/read/project structure.

## 3. Executable evidence

`computed_checked_result_match_payloads_lower_to_verified_ir` asserts:

- `lower_core` succeeds;
- independent `verify` returns no errors;
- the function has one typed spill local and three blocks;
- block 0 emits `CheckedAdd` then `WriteLocal`;
- both arm blocks begin with `ReadLocal` then `Project`.

Reproducible validator:

```powershell
$env:RUSTUP_TOOLCHAIN = '1.98.0-x86_64-pc-windows-gnu'
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test --locked --offline -p sico-ir --test core_lowering computed_checked_result_match_payloads_lower_to_verified_ir
```

Evidence class: `internal-fixture`, Windows x64 GNU. This is Rust-oracle
runtime test evidence, not Sico self-host match-lowering support.

## 4. Residuals

At this step boundary the Sico-written frontend still refused match
control flow. STEP-0215 subsequently mirrors this exact,
verifier-proven all-return checked-Result shape. No bootstrap, UI, or GUI
application claim advances in this investigation step.
