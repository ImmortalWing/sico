# STEP-0134: fixed-width checked multiply and divide

> - status: complete
> - phase: M14 (gap-closing STEP per RFC-0038 §4)
> - started: 2026-09-06
> - completed: 2026-09-06
> - owners: autonomous-agent
> - contract: RFC-0038 (accepted 2026-09-05); profile item 5 (fixed-width
>   arithmetic: checked `+ - * /`, comparisons, bit operations)

## 1. What was implemented

The remaining two arithmetic slices of RFC-0038 profile item 5, end to end:

- **Source surface**: `I64.checked_mul/checked_div` and
  `U64.checked_mul/checked_div`, each returning
  `Result[I64|U64, NumericError]` like the existing add/sub.
- **Semantics**: `infer_fixed_width_call` accepts the two operations with
  the same operand-type rule (both operands share the receiver type).
- **IR**: `CheckedMul`/`CheckedDiv` pure operations; the existing generic
  `verify_checked_fixed` rule covers them unchanged.
- **Codegen**:
  - `checked_mul` computes the wrapping product and detects overflow by
    dividing back (`p / a != b` for `a != 0`, with `a == 0` and
    `a == -1`/`b == MIN` pre-empted so no trapping division is ever
    executed). Soundness argument is recorded at
    `emit_checked_mul` in `sico-codegen-wasm/src/lib.rs`: the discarded
    high part shifts the quotient by `k*2^64/a` with `|k*2^64/a| > 1`
    whenever `k != 0`, so truncation cannot hide a real overflow. I64
    error payloads distinguish direction (same-sign overflow /
    mixed-sign underflow), matching the add/sub convention.
  - `checked_div` reports `NumericError.division-by-zero` for a zero
    divisor and `overflow` for `i64::MIN / -1` (the only signed-overflow
    case); every other division is exact and trap-free.
  - `NumericError` gains the additive third case `division-by-zero`
    (tag 2). The enum lives only in generated component type exports —
    the frozen `wit/script-profile-v0` world never names it — and the
    runtime local form (`[i32 tag, i64 payload]`) is unchanged, so this
    is not a WIT contract change.
- **Frozen snapshots**: the `fixed-width-core` artifact stays
  byte-identical; `fixed-width-component` gains exactly 17 bytes (the new
  enum case name + count) and the new hex line is appended to
  `tests/wasm/artifacts.hex` (append-only snapshot convention). Frozen
  IR lowering snapshots untouched (additive name-map entries only).

## 2. End-to-end evidence

`tests/end-to-end/checked-mul-div.sico` — 19 guest-asserted cases over a
flat failure-accumulator shape (continuing match arms + `if`/`set`,
itself a STEP-0130 shape probe): exact products/quotients for both types,
`x*0`, `MAX*1`, `MAX*2` overflow, `MIN*-1` overflow, `(MIN/2)*3`
underflow, `2^32 * 2^32` U64 overflow, division by zero for both types,
`MIN / -1`, and `U64 MAX / 1`. Prints `mul-div-ok`; covered by
`runner/sico-runner/tests/checked_mul_div.rs` (deterministic result +
repeated-run isolation).

## 3. Residuals

None for this slice. Profile item 5 is now fully executable
(`+ - * /`, comparisons, bit operations).

## 4. Validation

- `cargo test -p sico-ir -p sico-codegen-wasm -p sico-semantics --all-targets` green.
- Runner workspace `--test checked_mul_div -- --test-threads=1`: 2/2 green.
- Root workspace full `cargo test --locked --offline --workspace --all-targets --all-features` green.
- `cargo fmt --all -- --check` clean; `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` clean.
- `tools/validate-step-0131.ps1` green (matrix fixture `fixed-width-arithmetic` row updated in place); `git diff --check` clean.
