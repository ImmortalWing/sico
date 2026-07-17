# General executable codegen v0 evidence

> - date: 2026-07-17
> - step: STEP-0078
> - status: complete
> - runtime: Wasmtime 46.0.1 (`823d1b8f2`, 2026-06-24)

## Implemented boundary

The verified IR backend now emits direct calls and arbitrary verified block graphs through a deterministic dispatcher loop. Function IDs are one-based in IR and are mapped explicitly to zero-based Wasm indices. Block IDs are stored in a private `i32` dispatcher local; returns leave the dispatcher, while jump, branch and match select the next verified block. Back-edges use the same path and rely on the Runtime fuel/timeout policy for non-termination.

Internal aggregates are local-only in this step:

- `Construct` records named field/value pairs and flattens their slots;
- `Project` copies a known local field layout;
- `Variant` stores a deterministic module-local tag followed by flattened payload slots;
- Boolean, wildcard and variant-tag match decisions are executable;
- payload bindings/inspection and aggregate parameters/results/calls fail with typed `CodegenError::Unsupported`.

This representation is not a Canonical ABI and does not claim Text, Bytes, List, record or general Result interoperability.

## Deterministic and mutation evidence

- Core and Component fixtures are byte-identical across repeated compilation and pass `wasmparser` validation.
- The fixture catches the one-based IR/zero-based Wasm call-index mismatch by calling function 1 from function 2.
- Verifier mutations reject duplicate constructor field names and variant patterns applied to Boolean values.
- Backend mutation preserves a typed refusal for match binding transport.
- Existing arbitrary-precision `Int` dynamic parameter/call refusals remain in place.

## Runtime oracle

`tools/validate-core-wasm-runtime.mjs` executes:

| Case | Expected |
|---|---:|
| direct `I64` identity call | `123` |
| two-jump chain | `-9` |
| Boolean match, true | `41` |
| Boolean match, false | `42` |
| local record construct/project | `22` |
| local variant/match | `1` |
| exiting CFG back-edge | `7` |

`tools/validate-step-0078.ps1` repeats all seven cases in exact Wasmtime 46.0.1. It invokes the non-exiting side of the same back-edge with 10,000 fuel and requires `all fuel consumed by WebAssembly`, proving that cyclic CFG emission is valid while termination remains Runtime-bounded.

## Reproduction

```powershell
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
cargo test --offline --locked -p sico-ir -p sico-codegen-wasm
.\tools\validate-step-0078.ps1
```

The repository pins Rust 1.97.1. This workstation currently has the 1.97.1 MSVC compiler but no MSVC linker, and the installed GNU toolchain is 1.97.0; the final workspace gate therefore records the exact executable toolchain used rather than implying a 1.97.1 GNU run.
