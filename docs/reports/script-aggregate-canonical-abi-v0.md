# Script aggregate Canonical ABI v0 evidence

> - date: 2026-07-17
> - step: STEP-0079
> - status: complete
> - runtime: Wasmtime 46.0.1 (`823d1b8f2`, 2026-06-24)

## Implemented boundary

The backend gains `compile_script_program`, which compiles a verified module containing exactly one `run(input: ScriptInput) -> Result[ScriptOutput, ScriptError]` entry (plus scalar helper functions) into a deterministic `sico:script/program@0.1.0` Program Component. The accepted aggregate shapes are strict UTF-8 Text (`string`), arbitrary Bytes (`list<u8>`), bounded `list<string>`, the `ScriptInput`/`ScriptOutput`/`ScriptError` records, the `script-error-code` enum and the boundary `result`; every shape has exactly one documented Core and Canonical ABI representation in [`script-canonical-abi-layout-v0.md`](../development/script-canonical-abi-layout-v0.md), derived from the embedded frozen WIT rather than hand-coded offsets.

The generated core module owns one bounded arena:

- static aggregate literals live in a deterministic data segment; the arena base follows it and the ceiling is the 64 MiB (1,024-page) declared memory;
- `alloc`/`cabi_realloc` use checked 32-bit arithmetic and trap on wrap or ceiling crossing;
- `run` receives the flattened `script-input` (four i32 values) and returns a pointer to the 32-byte result area; result payload bytes are copied into the arena so result buffers never alias caller-supplied parameter storage;
- `cabi_post_run` deterministically resets the arena bump pointer (idempotent cleanup), and the Component lifts `run` with `utf8`/`memory`/`realloc`/`post-return` canonical options.

Internal record/variant values remain flat locals (STEP-0078); the Canonical ABI applies only at the Component boundary. Aggregate helper-function parameters, aggregate call transport, `list` element access and non-boundary aggregates remain typed refusals.

## Independent host helpers

`sico_runtime::canonical` reimplements the same byte layout without any engine dependency: `lower_input`/`lift_input` and `lower_result`/`lift_result` over a bounded arena with M8 bounds (1,024 arguments, 64 KiB per argument, 1 MiB total argument bytes, 8 MiB channels, 64 KiB error message) enforced before any write. 10,000 seeded buffer roundtrips are byte/value exact; malformed-memory mutations (invalid UTF-8, invalid result/enum discriminants, misalignment, out-of-bounds and overflowing pointer/length, truncated buffers) produce typed refusals; cleanup is deterministic, idempotent and leaves repeated same-shape calls at an identical high-water mark.

## Exact-engine evidence

`prototypes/script-abi-roundtrip` (run by `tools/validate-step-0079.ps1`, JSON in `target/evidence/step-0079-script-abi-roundtrip.json`) executes compiler-generated Components on Wasmtime 46.0.1. Component identities are deterministic: the echo Program Component is 900 bytes (SHA-256 `218cb665554f0f334abbd4236af337f543a19eae629554877aa12994e726ff29`), the error fixture 850 bytes (SHA-256 `b2c3f47e867af24a7ea4a41a9dce9614e79221a95ac73c6512436a06f83347b7`), each byte-identical across rebuilds and pinned in `tests/wasm/artifacts.hex`.

- 10,000 seeded boundary roundtrips on one live instance — stdout/stderr/exit value exact for Unicode arguments and arbitrary binary stdin including NUL and non-UTF-8 bytes;
- a repeated-call cleanup oracle: 512 × 1 MiB plus 8 × 4 MiB calls on one instance, which would cross the 64 MiB arena ceiling without working post-return cleanup;
- a boundary-size case: 1,024 arguments and an 8 MiB stdin roundtrip;
- the error fixture returns `domain-error`/`rejected` through the real `result` lift;
- six malicious-memory fixtures (invalid result discriminant, invalid enum discriminant, invalid UTF-8 message, out-of-bounds payload pointer, overflowing payload length, misaligned result area) all fail closed at the engine lift.

## Honest limits

- Argument list element content is verified through the independent host helper and through the STEP-0076 hard-coded guests; compiler-generated guests reflect `list<string>` element content only after list element access lowering lands with the standard library (STEP-0083). Count and transport are covered by the frozen boundary.
- Source-level Script syntax, adapter composition, manifest v1, caches and runner policy are STEP-0080–0082 and are not claimed here.
- STEP-0078 scalar/call/CFG behavior and the full workspace regression remain green (`cargo test --offline --locked --workspace --all-targets --all-features`).

## Reproduction

```powershell
$env:RUSTUP_TOOLCHAIN = '1.97.0-x86_64-pc-windows-gnu'
cargo test --offline --locked -p sico-codegen-wasm -p sico-runtime
.\tools\validate-step-0079.ps1
```

The Wasmtime crate harness requires the MSVC toolchain on this host (the pinned GNU toolchain cannot link it); the validation script enters the VS 2022 Build Tools environment for the harness only and keeps workspace gates on the recorded GNU toolchain.
