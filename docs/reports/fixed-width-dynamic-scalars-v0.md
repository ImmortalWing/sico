# Fixed-width dynamic scalar evidence v0

> - status: verified implementation evidence
> - date: 2026-07-17
> - related step: STEP-0077
> - evidence boundary: semantics through direct Core Wasm and numeric Component results

## Summary

STEP-0077 adds separate `I64` and `U64` values without changing arbitrary-precision `Int`. Explicit literal construction is range checked, implicit `Int` narrowing is rejected, and fixed-width addition/subtraction returns `Result[fixed, NumericError]`. `NumericError` distinguishes `overflow` from `underflow`; no checked path wraps silently or converts failure into an untyped trap.

The same signedness contract now exists in semantic analysis, verified IR, deterministic Core Wasm and Component exports. Direct Core Wasm returns `(tag, payload)` for runtime testing. Component exports use Canonical ABI indirect results and expose actual `result<s64|u64, numeric-error>` values.

## Frozen source and IR surface

| Purpose | Source | IR |
|---|---|---|
| explicit literal | `I64.literal(Int)` / `U64.literal(Int)` | `ConstI64` / `ConstU64` |
| checked addition | `I64.checked_add` / `U64.checked_add` | `CheckedAdd` |
| checked subtraction | `I64.checked_sub` / `U64.checked_sub` | `CheckedSub` |
| equality | `I64.equal` / `U64.equal` | `EqualFixed` |
| ordering | `I64.less_than` / `U64.less_than` | `LessFixed` |

All binary operations require exactly two same-signedness operands. Checked arithmetic results are exactly `Result[I64|U64, NumericError]`; comparisons return `Bool`. The verifier independently rejects mixed signedness and forged result types.

## Runtime evidence

`tools/validate-core-wasm-runtime.mjs` loads the committed Core Wasm snapshot with Node v25.1.0. A deterministic 64-bit generator creates 2,048 operand pairs. Each pair executes signed/unsigned checked add, checked sub, equality and less-than, for 16,384 generated observations compared with JavaScript `BigInt` mathematical oracles. Six explicit min/max/zero boundary cases are checked separately.

`tools/validate-step-0077.ps1` then materializes the committed Component snapshot and invokes exact Wasmtime 46.0.1 in 11 separate cases. Representative outputs are:

```text
i64-checked-add(40, 2)                         -> ok(42)
i64-checked-add(9223372036854775807, 1)       -> err(overflow)
i64-checked-add(-9223372036854775808, -1)     -> err(underflow)
u64-checked-add(18446744073709551615, 1)      -> err(overflow)
u64-checked-sub(0, 1)                         -> err(underflow)
i64-less-than(-1, 0)                          -> true
u64-less-than(18446744073709551615, 0)        -> false
```

The validator terminates with:

```text
STEP_0077_OK runtime=wasmtime 46.0.1 (823d1b8f2 2026-06-24) component_cases=11 core_properties=2048x8 typed_failures=overflow,underflow
```

## Artifact and local timing measurements

| Artifact | Bytes |
|---|---:|
| direct fixed-width Core Wasm | 456 |
| fixed-width Component | 1,209 |

Ten complete validator runs, each including the Node oracle and 11 Wasmtime process launches, measured 333.006 ms minimum, 354.206 ms median and 460.088 ms nearest-rank P95/max. These values measure a regression harness, not guest startup or an M8 SLA.

## Canonical ABI finding

A Component `result<s64|u64, enum>` flattens to more than the one direct result allowed for a synchronous lifted export. The validator correctly rejected an initial direct `(i32, i64)` lift because a memory option was required. The final Component-specific Core module exports one-page private memory, stores the result discriminant at offset 0 and the aligned payload at offset 8, returns pointer 0, and supplies that memory to `canon lift`. Pointer-free numeric results need no realloc or post-return hook.

## Evidence boundary

Verified here:

- explicit literal range and intrinsic arity checks;
- no implicit `Int` to `I64/U64` conversion;
- verified operand/result signedness;
- dynamic fixed-width parameters, locals and results;
- typed overflow/underflow and signed/unsigned comparisons;
- deterministic, validator-clean Core Wasm and Components;
- real Node and Wasmtime execution.

Not verified here:

- runtime arbitrary-precision `Int` conversion or storage;
- general function calls, loops, jumps or match codegen;
- general record/list/string/result Canonical ABI;
- async, resources, Script adapter packaging or runner integration;
- non-Windows Runtime behavior.

## Reproduce

```powershell
$env:RUSTUP_TOOLCHAIN = '1.97.1-x86_64-pc-windows-gnu'
cargo test --offline --locked -p sico-semantics -p sico-ir -p sico-codegen-wasm
.\tools\validate-step-0077.ps1
```

## Links

- [`STEP-0077`](../steps/STEP-0077-fixed-width-dynamic-scalars.md)
- [`RFC-0029`](../rfc/RFC-0029-script-profile-v0.md)
- [`M8 plan`](../plans/M8-script-profile.md)
- [`STEP-0078`](../steps/STEP-0078-general-executable-control-codegen.md)
